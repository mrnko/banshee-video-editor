use banshee_ai::{fetch_organization_cost, ModelOption, OpenAiClient, RECOMMENDED_MODELS};
use banshee_domain::{
    default_presets, AnalysisSettings, AnalysisSource, Asset, AssetFolder, AssetKind,
    CandidateClip, CropMode, CropSettings, EditDecisionList, JobStatus, Project, ProjectStatus,
    RenderJob, TrackVisibility, UpscaleMode,
};
use banshee_media::MediaTools;
use banshee_storage::{delete_secret, get_secret, set_secret, AppSettings, DashboardStats, Store};
use banshee_upscaler::Upscaler;
use chrono::Utc;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

struct AppState {
    store: Arc<Store>,
    media: Option<MediaTools>,
    upscaler: Upscaler,
    last_cost_sync: Mutex<Option<Instant>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapData {
    projects: Vec<Project>,
    assets: Vec<Asset>,
    asset_folders: Vec<AssetFolder>,
    settings: AppSettings,
    stats: DashboardStats,
    presets: Vec<banshee_domain::RenderPreset>,
    ffmpeg_available: bool,
    upscaler_available: bool,
    api_key_configured: bool,
    version: &'static str,
    portable: bool,
}

type CommandResult<T> = Result<T, String>;

fn error_message(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn expose_media(app: &AppHandle, path: impl AsRef<Path>) {
    let _ = app.asset_protocol_scope().allow_file(path);
}

#[tauri::command]
fn bootstrap(app: AppHandle, state: State<'_, AppState>) -> CommandResult<BootstrapData> {
    let projects = state.store.projects().map_err(error_message)?;
    let assets = state.store.assets().map_err(error_message)?;
    for project in &projects {
        expose_media(&app, &project.source.path);
        if let Some(thumbnail) = &project.thumbnail_path {
            expose_media(&app, thumbnail);
        }
        if let Ok(candidates) = state.store.candidates(&project.id) {
            for candidate in candidates {
                if let Some(thumbnail) = candidate.thumbnail_path {
                    expose_media(&app, thumbnail);
                }
            }
        }
    }
    for asset in &assets {
        expose_media(&app, &asset.path);
        if let Some(thumbnail) = &asset.thumbnail_path {
            expose_media(&app, thumbnail);
        }
    }
    Ok(BootstrapData {
        projects,
        assets,
        asset_folders: state.store.asset_folders().map_err(error_message)?,
        settings: state.store.settings().map_err(error_message)?,
        stats: state.store.dashboard_stats().map_err(error_message)?,
        presets: default_presets(),
        ffmpeg_available: state.media.is_some(),
        upscaler_available: state.upscaler.available(),
        api_key_configured: get_secret("openai-api-key").is_some(),
        version: env!("CARGO_PKG_VERSION"),
        portable: std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("banshee-portable.marker")))
            .is_some_and(|marker| marker.exists()),
    })
}

#[tauri::command]
async fn import_video(
    path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Project> {
    let media = state.media.as_ref().ok_or("FFmpeg не знайдено")?;
    let source = media.probe(Path::new(&path)).await.map_err(error_message)?;
    let stem = Path::new(&path)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("Нове відео");
    let id = Uuid::new_v4().to_string();
    let project_dir = state.store.project_dir(&id).map_err(error_message)?;
    let thumbnail = project_dir.join("thumbnails").join("source.jpg");
    let thumbnail_path = media
        .generate_thumbnail(Path::new(&path), 0.5, &thumbnail)
        .await
        .ok()
        .and(thumbnail.is_file().then(|| thumbnail.to_string_lossy().into_owned()));
    let project = Project {
        id,
        name: stem.into(),
        source,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        status: ProjectStatus::Imported,
        thumbnail_path,
    };
    state.store.save_project(&project).map_err(error_message)?;
    expose_media(&app, &project.source.path);
    if let Some(thumbnail) = &project.thumbnail_path {
        expose_media(&app, thumbnail);
    }
    Ok(project)
}

#[tauri::command]
fn project_candidates(
    project_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Vec<CandidateClip>> {
    let candidates = state.store.candidates(&project_id).map_err(error_message)?;
    for candidate in &candidates {
        if let Some(thumbnail) = &candidate.thumbnail_path {
            expose_media(&app, thumbnail);
        }
    }
    Ok(candidates)
}

#[tauri::command]
async fn analyze_project(
    project_id: String,
    settings: AnalysisSettings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Vec<CandidateClip>> {
    let media = state.media.as_ref().ok_or("FFmpeg не знайдено")?;
    let mut project = state.store.project(&project_id).map_err(error_message)?;
    project.status = ProjectStatus::Analyzing;
    project.updated_at = Utc::now();
    state.store.save_project(&project).map_err(error_message)?;
    let _ = app.emit(
        "analysis-progress",
        serde_json::json!({"projectId": project_id, "progress": 8, "stage": "Підготовка proxy"}),
    );
    let project_dir = state
        .store
        .project_dir(&project.id)
        .map_err(error_message)?;
    let mut clips = media
        .analyze_local(
            &project.id,
            &project.source,
            &settings,
            &project_dir.join("thumbnails"),
        )
        .await
        .map_err(error_message)?;
    let _ = app.emit("analysis-progress", serde_json::json!({"projectId": project_id, "progress": 68, "stage": "Оцінювання кандидатів"}));
    if settings.use_openai {
        if let Some(key) = get_secret("openai-api-key") {
            if let Ok(client) = OpenAiClient::new(key) {
                let audio_path = project_dir.join("temp").join("speech-analysis.m4a");
                let mut transcript = None;
                if let Ok(Some(path)) = media
                    .extract_speech_audio(&project.source, &audio_path)
                    .await
                {
                    let _ = app.emit("analysis-progress", serde_json::json!({"projectId": project_id, "progress": 76, "stage": "Транскрипція мовлення"}));
                    if let Ok((text, usage)) = client
                        .transcribe(&project.id, &path, project.source.duration_seconds)
                        .await
                    {
                        transcript = Some(text);
                        state.store.add_usage(&usage).map_err(error_message)?;
                    }
                }
                match client
                    .rank_candidates(
                        &project.id,
                        &settings.model,
                        &settings.prompt,
                        &clips,
                        transcript.as_deref(),
                    )
                    .await
                {
                    Ok((rankings, usage)) => {
                        let share = if clips.is_empty() {
                            0.0
                        } else {
                            usage.estimated_cost_usd / clips.len() as f64
                        };
                        for clip in &mut clips {
                            if let Some(ranking) =
                                rankings.iter().find(|ranking| ranking.id == clip.id)
                            {
                                clip.score = ranking.score;
                                clip.reason = ranking.reason.clone();
                                clip.analysis_source = AnalysisSource::OpenAi;
                                clip.estimated_cost_usd = share;
                            }
                        }
                        state.store.add_usage(&usage).map_err(error_message)?;
                    }
                    Err(error) => {
                        let _ = app.emit(
                            "analysis-warning",
                            format!("OpenAI недоступний, використано локальний аналіз: {error}"),
                        );
                    }
                }
            }
        }
    }
    clips.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    state
        .store
        .save_candidates(&project.id, &clips)
        .map_err(error_message)?;
    for clip in &clips {
        if let Some(thumbnail) = &clip.thumbnail_path {
            expose_media(&app, thumbnail);
        }
    }
    project.status = ProjectStatus::Ready;
    project.updated_at = Utc::now();
    state.store.save_project(&project).map_err(error_message)?;
    let _ = app.emit(
        "analysis-progress",
        serde_json::json!({"projectId": project_id, "progress": 100, "stage": "Готово"}),
    );
    Ok(clips)
}

#[tauri::command]
fn save_edit(edit: EditDecisionList, state: State<'_, AppState>) -> CommandResult<()> {
    validate_edit(&edit)?;
    state.store.save_edit(&edit).map_err(error_message)
}

#[tauri::command]
fn load_edit(
    candidate_id: String,
    state: State<'_, AppState>,
) -> CommandResult<Option<EditDecisionList>> {
    state.store.edit(&candidate_id).map_err(error_message)
}

#[tauri::command]
async fn render_clip(
    project_id: String,
    candidate_id: String,
    preset_id: String,
    output_path: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<RenderJob> {
    let media = state.media.as_ref().ok_or("FFmpeg не знайдено")?;
    let project = state.store.project(&project_id).map_err(error_message)?;
    let candidate = state
        .store
        .candidates(&project_id)
        .map_err(error_message)?
        .into_iter()
        .find(|c| c.id == candidate_id)
        .ok_or("Кліп не знайдено")?;
    let edit = state
        .store
        .edit(&candidate.id)
        .map_err(error_message)?
        .unwrap_or_else(|| default_edit(&candidate));
    let preset = default_presets()
        .into_iter()
        .find(|p| p.id == preset_id)
        .ok_or("Профіль якості не знайдено")?;
    let project_dir = state
        .store
        .project_dir(&project_id)
        .map_err(error_message)?;
    let path = output_path.map(PathBuf::from).unwrap_or_else(|| {
        project_dir
            .join("exports")
            .join(format!("banshee-{}.mp4", &candidate.id[..8]))
    });
    let assets: HashMap<_, _> = state
        .store
        .assets()
        .map_err(error_message)?
        .into_iter()
        .filter(|a| !a.missing)
        .map(|a| (a.id, a.path))
        .collect();
    let id = Uuid::new_v4().to_string();
    let _ = app.emit(
        "render-progress",
        serde_json::json!({"id": id, "progress": 5, "stage": "Підготовка рендера"}),
    );
    let mut render_source = project.source.clone();
    let mut render_edit = edit.clone();
    let mut warning = None;
    let effective_width = (project.source.width as f32)
        .min(project.source.height as f32 * 9.0 / 16.0 / edit.crop.zoom.max(1.0));
    let wants_upscale = !matches!(preset.ai_upscale, UpscaleMode::Off)
        && !matches!(edit.crop.mode, CropMode::BlurredBackground)
        && effective_width + 1.0 < preset.width as f32;
    let upscale_dir = project_dir.join("jobs").join(&id).join("upscale");
    if wants_upscale && state.upscaler.available() {
        let fps = preset
            .max_fps
            .min(project.source.fps.round().max(24.0) as u32);
        let scale = if effective_width * 2.0 >= preset.width as f32 {
            2
        } else {
            4
        };
        let input_frames = upscale_dir.join("input");
        let output_frames = upscale_dir.join("output");
        let intermediate = upscale_dir.join("upscaled.mkv");
        let upscale_result = async {
            let _ = app.emit(
                "render-progress",
                serde_json::json!({"id": id, "progress": 15, "stage": "Підготовка AI-upscale"}),
            );
            media
                .extract_vertical_frames(&project.source, &edit, fps, &input_frames)
                .await?;
            let _ = app.emit(
                "render-progress",
                serde_json::json!({"id": id, "progress": 35, "stage": "AI-upscale кадрів"}),
            );
            state
                .upscaler
                .enhance_frames(&input_frames, &output_frames, &preset.ai_upscale, scale)
                .await?;
            let _ = app.emit(
                "render-progress",
                serde_json::json!({"id": id, "progress": 55, "stage": "Збирання lossless-кліпу"}),
            );
            media
                .assemble_upscaled_clip(
                    &project.source,
                    edit.trim_start,
                    edit.trim_end - edit.trim_start,
                    fps,
                    &output_frames,
                    &intermediate,
                )
                .await
        }
        .await;
        match upscale_result {
            Ok(source) => {
                render_source = source;
                let original_start = edit.trim_start;
                render_edit.trim_start = 0.0;
                render_edit.trim_end = edit.trim_end - original_start;
                for range in &mut render_edit.removed_ranges {
                    range.start -= original_start;
                    range.end -= original_start;
                }
                render_edit.crop.zoom = 1.0;
            }
            Err(error) => {
                warning = Some(format!(
                    "AI-upscale недоступний: {error}. Використано якісний FFmpeg fallback."
                ))
            }
        }
    } else if wants_upscale {
        warning = Some("Real-ESRGAN не встановлено — використано якісний Lanczos fallback.".into());
    }
    let render_result = media
        .render(&render_source, &render_edit, &preset, &assets, &path)
        .await;
    if upscale_dir.exists() {
        let _ = std::fs::remove_dir_all(&upscale_dir);
    }
    render_result.map_err(error_message)?;
    state
        .store
        .record_render(&project_id)
        .map_err(error_message)?;
    expose_media(&app, &path);
    let _ = app.emit(
        "render-progress",
        serde_json::json!({"id": id, "progress": 100, "stage": "Готово"}),
    );
    Ok(RenderJob {
        id,
        project_id,
        candidate_id,
        output_path: path.to_string_lossy().into_owned(),
        status: JobStatus::Completed,
        progress: 100.0,
        warning,
    })
}

#[tauri::command]
async fn import_asset(
    path: String,
    folder_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Asset> {
    let source = Path::new(&path);
    if !source.is_file() {
        return Err("Файл ресурсу не знайдено".into());
    }
    let extension = source
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = match extension.as_str() {
        "mp4" | "mov" | "mkv" | "webm" => AssetKind::Video,
        "mp3" | "wav" | "aac" | "m4a" | "flac" => AssetKind::Audio,
        "png" | "jpg" | "jpeg" | "webp" => AssetKind::Image,
        _ => return Err("Непідтримуваний тип ресурсу".into()),
    };
    let id = Uuid::new_v4().to_string();
    let thumbnail_path = if matches!(kind, AssetKind::Video) {
        let thumbnail = state
            .store
            .library_dir()
            .map_err(error_message)?
            .join("thumbnails")
            .join(format!("{id}.jpg"));
        match state.media.as_ref() {
            Some(media) if media.generate_thumbnail(source, 0.5, &thumbnail).await.is_ok() => {
                expose_media(&app, &thumbnail);
                Some(thumbnail.to_string_lossy().into_owned())
            }
            _ => None,
        }
    } else {
        None
    };
    let asset = Asset {
        id,
        folder_id,
        name: source
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("resource")
            .into(),
        path: source.to_string_lossy().into_owned(),
        kind,
        missing: false,
        thumbnail_path,
        created_at: Utc::now(),
    };
    state.store.add_asset(&asset).map_err(error_message)?;
    expose_media(&app, &asset.path);
    Ok(asset)
}

fn valid_name(value: &str, label: &str) -> CommandResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 {
        return Err(format!("Назва {label} має містити 1–80 символів"));
    }
    Ok(value.into())
}

#[tauri::command]
fn rename_project(project_id: String, name: String, state: State<'_, AppState>) -> CommandResult<Project> {
    let mut project = state.store.project(&project_id).map_err(error_message)?;
    project.name = valid_name(&name, "проєкту")?;
    project.updated_at = Utc::now();
    state.store.save_project(&project).map_err(error_message)?;
    Ok(project)
}

#[tauri::command]
fn delete_project(project_id: String, state: State<'_, AppState>) -> CommandResult<()> {
    state.store.delete_project(&project_id).map_err(error_message)
}

#[tauri::command]
fn rename_candidate(candidate_id: String, name: String, state: State<'_, AppState>) -> CommandResult<CandidateClip> {
    let mut candidate = state.store.candidate(&candidate_id).map_err(error_message)?;
    candidate.name = Some(valid_name(&name, "моменту")?);
    state.store.save_candidate(&candidate).map_err(error_message)?;
    Ok(candidate)
}

#[tauri::command]
fn delete_candidate(candidate_id: String, state: State<'_, AppState>) -> CommandResult<()> {
    state.store.delete_candidate(&candidate_id).map_err(error_message)
}

#[tauri::command]
fn rename_asset(asset_id: String, name: String, state: State<'_, AppState>) -> CommandResult<Asset> {
    let mut asset = state.store.asset(&asset_id).map_err(error_message)?;
    asset.name = valid_name(&name, "ресурсу")?;
    state.store.add_asset(&asset).map_err(error_message)?;
    Ok(asset)
}

#[tauri::command]
fn delete_asset(asset_id: String, state: State<'_, AppState>) -> CommandResult<()> {
    state.store.delete_asset(&asset_id).map_err(error_message)
}

#[tauri::command]
fn create_asset_folder(name: String, state: State<'_, AppState>) -> CommandResult<AssetFolder> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err("Назва директорії має містити 1–80 символів".into());
    }
    let folder = AssetFolder {
        id: Uuid::new_v4().to_string(),
        name: name.into(),
        created_at: Utc::now(),
    };
    state
        .store
        .add_asset_folder(&folder)
        .map_err(error_message)?;
    Ok(folder)
}

#[tauri::command]
fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> CommandResult<()> {
    if !(5..=300).contains(&settings.autosave_seconds) {
        return Err("Інтервал autosave має бути 5–300 секунд".into());
    }
    state.store.save_settings(&settings).map_err(error_message)
}

#[tauri::command]
fn set_api_key(kind: String, value: String) -> CommandResult<()> {
    let account = match kind.as_str() {
        "openai" => "openai-api-key",
        "admin" => "openai-admin-key",
        _ => return Err("Невідомий тип ключа".into()),
    };
    if value.trim().is_empty() {
        delete_secret(account).map_err(error_message)
    } else {
        set_secret(account, value.trim()).map_err(error_message)
    }
}

#[tauri::command]
async fn list_openai_models() -> CommandResult<Vec<ModelOption>> {
    let Some(key) = get_secret("openai-api-key") else {
        return Ok(RECOMMENDED_MODELS
            .iter()
            .map(|id| ModelOption {
                id: (*id).into(),
                available: false,
                experimental: false,
                recommended: *id == "gpt-5.6-terra",
            })
            .collect());
    };
    OpenAiClient::new(key)
        .map_err(error_message)?
        .list_models()
        .await
        .map_err(error_message)
}

#[tauri::command]
async fn sync_openai_costs(state: State<'_, AppState>) -> CommandResult<f64> {
    {
        let last = state.last_cost_sync.lock().map_err(error_message)?;
        if let Some(instant) = *last {
            let remaining = Duration::from_secs(15 * 60).saturating_sub(instant.elapsed());
            if !remaining.is_zero() {
                return Err(format!(
                    "Повторна звірка доступна через {} хв.",
                    remaining.as_secs().div_ceil(60)
                ));
            }
        }
    }
    let key = get_secret("openai-admin-key").ok_or("Admin API key не налаштовано")?;
    let start = Utc::now().timestamp() - 30 * 24 * 60 * 60;
    let cost = fetch_organization_cost(&key, start)
        .await
        .map_err(error_message)?;
    *state.last_cost_sync.lock().map_err(error_message)? = Some(Instant::now());
    Ok(cost)
}

#[tauri::command]
fn dashboard_stats(state: State<'_, AppState>) -> CommandResult<DashboardStats> {
    state.store.dashboard_stats().map_err(error_message)
}

#[tauri::command]
fn reveal_file(path: String) -> CommandResult<()> {
    if !Path::new(&path).exists() {
        return Err("Файл не знайдено".into());
    }
    Command::new("explorer.exe")
        .arg(format!("/select,{path}"))
        .spawn()
        .map_err(error_message)?;
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn share_file(path: String, app: AppHandle) -> CommandResult<()> {
    use windows::{
        core::{factory, Interface, HSTRING},
        ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager},
        Foundation::TypedEventHandler,
        Storage::{IStorageItem, StorageFile},
        Win32::UI::Shell::IDataTransferManagerInterop,
    };
    use windows_collections::IIterable;

    if !Path::new(&path).is_file() {
        return Err("Файл не знайдено".into());
    }
    let interop: IDataTransferManagerInterop =
        factory::<DataTransferManager, IDataTransferManagerInterop>().map_err(error_message)?;
    let window = app
        .get_webview_window("main")
        .ok_or("Головне вікно не знайдено")?;
    let hwnd = window.hwnd().map_err(error_message)?;
    let manager: DataTransferManager =
        unsafe { interop.GetForWindow(hwnd) }.map_err(error_message)?;
    let shared_path = path.clone();
    let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
        TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
            move |_sender, args| {
                let storage_file =
                    StorageFile::GetFileFromPathAsync(&HSTRING::from(&shared_path))?.get()?;
                let storage_item: IStorageItem = storage_file.cast()?;
                let items: IIterable<IStorageItem> = vec![Some(storage_item)].into();
                let data = args.ok()?.Request()?.Data()?;
                data.Properties()?
                    .SetTitle(&HSTRING::from("Banshee Video Editor"))?;
                data.SetStorageItemsReadOnly(&items)?;
                Ok(())
            },
        );
    manager.DataRequested(&handler).map_err(error_message)?;
    unsafe { interop.ShowShareUIForWindow(hwnd) }.map_err(error_message)
}

fn validate_edit(edit: &EditDecisionList) -> CommandResult<()> {
    if edit.trim_start < 0.0 || edit.trim_end <= edit.trim_start {
        return Err("Невірний діапазон trim".into());
    }
    let mut ranges: Vec<_> = edit
        .removed_ranges
        .iter()
        .map(|range| {
            (
                range.start.max(edit.trim_start),
                range.end.min(edit.trim_end),
            )
        })
        .filter(|(start, end)| end > start)
        .collect();
    if ranges.len() != edit.removed_ranges.len() {
        return Err("Невірний інтервал видалення".into());
    }
    ranges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut covered = 0.0;
    let mut cursor = edit.trim_start;
    for (start, end) in ranges {
        if start > cursor {
            covered += end - start;
        } else if end > cursor {
            covered += end - cursor;
        }
        cursor = cursor.max(end);
    }
    if covered >= edit.trim_end - edit.trim_start - 0.1 {
        return Err("Не можна видалити весь кліп".into());
    }
    Ok(())
}

fn default_edit(candidate: &CandidateClip) -> EditDecisionList {
    EditDecisionList {
        project_id: candidate.project_id.clone(),
        candidate_id: candidate.id.clone(),
        trim_start: candidate.start_seconds,
        trim_end: candidate.end_seconds,
        removed_ranges: vec![],
        crop: CropSettings {
            mode: CropMode::Center,
            zoom: 1.0,
            center_x: 0.5,
            center_y: 0.5,
        },
        overlays: vec![],
        revision: 1,
        cut_points: vec![],
        fps: None,
        track_visibility: TrackVisibility::default(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::open_default().expect("Banshee storage initialization failed");
    let state = AppState {
        store: Arc::new(store),
        media: MediaTools::discover().ok(),
        upscaler: Upscaler::discover(),
        last_cost_sync: Mutex::new(None),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            import_video,
            project_candidates,
            analyze_project,
            save_edit,
            load_edit,
            render_clip,
            import_asset,
            create_asset_folder,
            rename_project,
            delete_project,
            rename_candidate,
            delete_candidate,
            rename_asset,
            delete_asset,
            save_settings,
            set_api_key,
            list_openai_models,
            sync_openai_costs,
            dashboard_stats,
            reveal_file,
            share_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Banshee Video Editor");
}
