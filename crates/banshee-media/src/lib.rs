use anyhow::{anyhow, bail, Context, Result};
use banshee_domain::{
    AnalysisSettings, AnalysisSource, CandidateClip, CropMode, EditDecisionList, Overlay,
    RenderPreset, SourceMedia, TimeRange,
};
use serde::Deserialize;
use std::{
    cmp::Ordering,
    collections::HashMap,
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct MediaTools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

impl MediaTools {
    pub fn discover() -> Result<Self> {
        let ffmpeg = discover_binary("FFMPEG_PATH", "ffmpeg.exe")?;
        let ffprobe = env::var_os("FFPROBE_PATH")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| {
                ffmpeg
                    .parent()
                    .map(|p| p.join("ffprobe.exe"))
                    .filter(|p| p.is_file())
            })
            .or_else(|| find_on_path("ffprobe.exe"))
            .context("ffprobe.exe не знайдено")?;
        Ok(Self { ffmpeg, ffprobe })
    }

    pub async fn probe(&self, path: &Path) -> Result<SourceMedia> {
        let output = Command::new(&self.ffprobe)
            .args([
                "-v",
                "error",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
            ])
            .arg(path)
            .output()
            .await?;
        if !output.status.success() {
            bail!("ffprobe не зміг прочитати відео")
        }
        let probe: Probe = serde_json::from_slice(&output.stdout)?;
        let video = probe
            .streams
            .iter()
            .find(|s| s.codec_type == "video")
            .context("Відеопотік не знайдено")?;
        let audio = probe.streams.iter().find(|s| s.codec_type == "audio");
        let duration_seconds = probe
            .format
            .duration
            .as_deref()
            .and_then(|v| v.parse().ok())
            .or_else(|| video.duration.as_deref().and_then(|v| v.parse().ok()))
            .unwrap_or_default();
        let fps = parse_rate(
            video
                .avg_frame_rate
                .as_deref()
                .or(video.r_frame_rate.as_deref())
                .unwrap_or("0/1"),
        );
        Ok(SourceMedia {
            path: path.to_string_lossy().into_owned(),
            file_name: path
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("video")
                .into(),
            duration_seconds,
            width: video.width.unwrap_or_default(),
            height: video.height.unwrap_or_default(),
            fps,
            video_codec: video.codec_name.clone().unwrap_or_else(|| "unknown".into()),
            audio_codec: audio.and_then(|s| s.codec_name.clone()),
            bitrate: probe
                .format
                .bit_rate
                .as_deref()
                .and_then(|v| v.parse().ok())
                .unwrap_or_default(),
        })
    }

    pub async fn analyze_local(
        &self,
        project_id: &str,
        source: &SourceMedia,
        settings: &AnalysisSettings,
        thumbnail_dir: &Path,
    ) -> Result<Vec<CandidateClip>> {
        std::fs::create_dir_all(thumbnail_dir)?;
        let metrics = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-nostdin", "-i"])
            .arg(&source.path)
            .args([
                "-vf",
                "fps=2,scale=320:-2,signalstats,metadata=print",
                "-an",
                "-f",
                "null",
                "-",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();
        let silence = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-nostdin", "-i"])
            .arg(&source.path)
            .args([
                "-af",
                "silencedetect=noise=-50dB:d=2",
                "-vn",
                "-f",
                "null",
                "-",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output();
        let (metrics_output, silence_output) = tokio::join!(metrics, silence);
        let metrics_output = metrics_output?;
        let silence_log = silence_output
            .map(|value| String::from_utf8_lossy(&value.stderr).into_owned())
            .unwrap_or_default();
        let silent_ranges = parse_silence_ranges(&silence_log, source.duration_seconds);
        let metrics_log = String::from_utf8_lossy(&metrics_output.stderr);
        let mut events = parse_signal_events(&metrics_log);
        for event in &mut events {
            if silent_ranges
                .iter()
                .any(|range| event.time >= range.0 && event.time <= range.1)
            {
                event.score *= 0.88;
            }
        }
        if events.is_empty() {
            let count = settings
                .moment_count
                .unwrap_or_else(|| ((source.duration_seconds / 180.0).round() as u8).clamp(2, 8));
            events = (1..=count)
                .map(|i| SceneEvent {
                    time: source.duration_seconds * i as f64 / (count as f64 + 1.0),
                    score: 0.45,
                    motion: 0.0,
                    saturation: 0.0,
                })
                .collect();
        }
        let target_count = settings
            .moment_count
            .unwrap_or_else(|| ((source.duration_seconds / 180.0).round() as u8).clamp(2, 8))
            as usize;
        events.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        let mut selected: Vec<SceneEvent> = Vec::new();
        for event in events {
            if event.time < 2.0 || event.time > source.duration_seconds - 2.0 {
                continue;
            }
            if selected
                .iter()
                .any(|item| (item.time - event.time).abs() < settings.min_duration_seconds as f64)
            {
                continue;
            }
            selected.push(event);
            if selected.len() >= target_count.saturating_mul(2).clamp(4, 16) {
                break;
            }
        }
        selected.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        selected.truncate(target_count);
        let duration = ((settings.min_duration_seconds + settings.max_duration_seconds) as f64
            / 2.0)
            .clamp(15.0, 60.0);
        let mut clips = Vec::new();
        for (index, event) in selected.into_iter().enumerate() {
            let start = (event.time - duration * 0.22)
                .max(0.0)
                .min((source.duration_seconds - duration).max(0.0));
            let end = (start + duration).min(source.duration_seconds);
            let id = Uuid::new_v4().to_string();
            let thumbnail = thumbnail_dir.join(format!("candidate-{:02}.jpg", index + 1));
            let _ = self
                .thumbnail(
                    Path::new(&source.path),
                    start + (end - start) * 0.35,
                    &thumbnail,
                )
                .await;
            clips.push(CandidateClip {
                id, project_id: project_id.into(), start_seconds: start, end_seconds: end,
                score: (55.0 + event.score * 40.0).clamp(0.0, 99.0),
                reason: format!("Локально знайдено високу динаміку кадру ({:.0}%) і візуальну насиченість ({:.0}%); фрагмент починається близько до піку дії.", (event.motion / 28.0 * 100.0).clamp(0.0, 100.0), (event.saturation / 70.0 * 100.0).clamp(0.0, 100.0)),
                thumbnail_path: thumbnail.is_file().then(|| thumbnail.to_string_lossy().into_owned()),
                estimated_cost_usd: 0.0, analysis_source: AnalysisSource::Local,
            });
        }
        clips.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        Ok(clips)
    }

    pub async fn extract_speech_audio(
        &self,
        source: &SourceMedia,
        output: &Path,
    ) -> Result<Option<PathBuf>> {
        if source.audio_codec.is_none() {
            return Ok(None);
        }
        let detect = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-nostdin", "-i"])
            .arg(&source.path)
            .args(["-af", "volumedetect", "-vn", "-f", "null", "-"])
            .output()
            .await?;
        let log = String::from_utf8_lossy(&detect.stderr);
        let mean = log
            .lines()
            .find_map(|line| line.split("mean_volume:").nth(1))
            .and_then(|tail| tail.trim().split_whitespace().next())
            .and_then(|value| value.parse::<f32>().ok());
        if mean.is_none() || mean.unwrap_or(-100.0) < -55.0 {
            return Ok(None);
        }
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let status = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
            .arg(&source.path)
            .args([
                "-vn", "-ac", "1", "-ar", "16000", "-c:a", "aac", "-b:a", "32k",
            ])
            .arg(output)
            .status()
            .await?;
        if !status.success() {
            return Ok(None);
        }
        Ok(Some(output.to_path_buf()))
    }

    async fn thumbnail(&self, input: &Path, time: f64, output: &Path) -> Result<()> {
        let status = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                &format!("{time:.3}"),
                "-i",
            ])
            .arg(input)
            .args(["-frames:v", "1", "-vf", "scale=640:-2", "-q:v", "2"])
            .arg(output)
            .status()
            .await?;
        if !status.success() {
            bail!("Не вдалося створити preview")
        }
        Ok(())
    }

    pub async fn extract_vertical_frames(
        &self,
        source: &SourceMedia,
        edit: &EditDecisionList,
        fps: u32,
        output_dir: &Path,
    ) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let duration = (edit.trim_end - edit.trim_start).max(0.1);
        let zoom = edit.crop.zoom.clamp(1.0, 3.0);
        let filter = format!(
            "crop='min(iw,ih*9/16/{zoom})':'min(ih,iw*16/9/{zoom})':(iw-ow)/2:(ih-oh)/2,fps={}",
            fps.max(1)
        );
        let pattern = output_dir.join("frame-%08d.png");
        let result = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-y",
                "-ss",
                &format!("{:.3}", edit.trim_start),
                "-t",
                &format!("{duration:.3}"),
                "-i",
            ])
            .arg(&source.path)
            .args(["-an", "-vf", &filter])
            .arg(pattern)
            .output()
            .await?;
        if !result.status.success() {
            bail!(
                "Не вдалося підготувати кадри для AI-upscale: {}",
                tail(&String::from_utf8_lossy(&result.stderr), 12)
            )
        }
        Ok(())
    }

    pub async fn assemble_upscaled_clip(
        &self,
        source: &SourceMedia,
        start: f64,
        duration: f64,
        fps: u32,
        frames_dir: &Path,
        output: &Path,
    ) -> Result<SourceMedia> {
        let pattern = frames_dir.join("frame-%08d.png");
        let result = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-y",
                "-framerate",
            ])
            .arg(fps.max(1).to_string())
            .args(["-i"])
            .arg(pattern)
            .args([
                "-ss",
                &format!("{start:.3}"),
                "-t",
                &format!("{duration:.3}"),
                "-i",
            ])
            .arg(&source.path)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "1:a?",
                "-c:v",
                "ffv1",
                "-level",
                "3",
                "-c:a",
                "copy",
                "-shortest",
            ])
            .arg(output)
            .output()
            .await?;
        if !result.status.success() {
            bail!(
                "Не вдалося зібрати AI-upscale кліп: {}",
                tail(&String::from_utf8_lossy(&result.stderr), 12)
            )
        }
        self.probe(output).await
    }

    pub async fn available_encoder(&self) -> String {
        let output = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .output()
            .await;
        let text = output
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        for encoder in ["h264_nvenc", "h264_qsv", "h264_amf", "h264_mf", "libx264"] {
            if text.contains(encoder) {
                return encoder.into();
            }
        }
        "libx264".into()
    }

    pub async fn render(
        &self,
        source: &SourceMedia,
        edit: &EditDecisionList,
        preset: &RenderPreset,
        assets: &HashMap<String, String>,
        output: &Path,
    ) -> Result<Option<String>> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let duration = (edit.trim_end - edit.trim_start).max(0.1);
        let mut command = Command::new(&self.ffmpeg);
        command
            .args([
                "-hide_banner",
                "-nostdin",
                "-y",
                "-ss",
                &format!("{:.3}", edit.trim_start),
                "-t",
                &format!("{duration:.3}"),
                "-i",
            ])
            .arg(&source.path);
        let mut asset_indices = HashMap::new();
        for overlay in &edit.overlays {
            let asset_id = match overlay {
                Overlay::Audio { asset_id, .. } | Overlay::Video { asset_id, .. } => Some(asset_id),
                _ => None,
            };
            if let Some(asset_id) = asset_id {
                if !asset_indices.contains_key(asset_id) {
                    if let Some(path) = assets.get(asset_id) {
                        let index = asset_indices.len() + 1;
                        command.args(["-i", path]);
                        asset_indices.insert(asset_id.clone(), index);
                    }
                }
            }
        }
        let kept = kept_segments(edit);
        let (mut filter, video_input, source_audio_input) =
            cut_filter(&kept, source.audio_codec.is_some());
        let base = base_video_filter(
            &video_input,
            &edit.crop.mode,
            edit.crop.zoom,
            preset.width,
            preset.height,
        );
        if !filter.is_empty() {
            filter.push(';');
        }
        filter.push_str(&base);
        let mut current_video = "base".to_string();
        for overlay in &edit.overlays {
            match overlay {
                Overlay::Text {
                    id,
                    text,
                    color,
                    size,
                    x,
                    y,
                    start,
                    end,
                } => {
                    let next = format!("v{}", sanitize_label(id));
                    filter.push_str(&format!(";[{current_video}]drawtext=text='{}':fontcolor={}:fontsize={}:x=(w-text_w)*{:.4}:y=(h-text_h)*{:.4}:borderw=2:bordercolor=black@0.6:enable='between(t,{:.3},{:.3})'[{next}]", escape_drawtext(text), normalize_color(color), size, x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), start, end));
                    current_video = next;
                }
                Overlay::Video {
                    id,
                    asset_id,
                    start,
                    end,
                    x,
                    y,
                    scale,
                    chroma_key,
                } => {
                    if let Some(index) = asset_indices.get(asset_id) {
                        let prepared = format!("ov{}", sanitize_label(id));
                        let next = format!("v{}", sanitize_label(id));
                        let chroma = chroma_key
                            .as_ref()
                            .map(|key| {
                                format!(
                                    ",chromakey={}:{}:{},despill=type=green:mix={}",
                                    normalize_color(&key.color),
                                    key.similarity.clamp(0.01, 1.0),
                                    key.blend.clamp(0.0, 1.0),
                                    key.spill.clamp(0.0, 1.0)
                                )
                            })
                            .unwrap_or_default();
                        filter.push_str(&format!(";[{index}:v]setpts=PTS-STARTPTS+{start:.3}/TB,scale=iw*{:.3}:-2{chroma}[{prepared}];[{current_video}][{prepared}]overlay=x=(W-w)*{:.4}:y=(H-h)*{:.4}:enable='between(t,{start:.3},{end:.3})'[{next}]", scale.clamp(0.05, 2.0), x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)));
                        current_video = next;
                    }
                }
                Overlay::Audio { .. } => {}
            }
        }
        let audio_overlays: Vec<_> = edit
            .overlays
            .iter()
            .filter_map(|overlay| {
                if let Overlay::Audio {
                    asset_id,
                    start,
                    volume,
                    ..
                } = overlay
                {
                    asset_indices
                        .get(asset_id)
                        .map(|index| (*index, *start, *volume))
                } else {
                    None
                }
            })
            .collect();
        let audio_map = if audio_overlays.is_empty() {
            source_audio_input
                .map(|label| audio_map_spec(&label))
                .unwrap_or_else(|| "0:a?".to_string())
        } else {
            let mut labels = Vec::new();
            if let Some(label) = source_audio_input {
                filter.push_str(&format!(";[{label}]asetpts=PTS-STARTPTS[a0]"));
                labels.push("[a0]".to_string());
            }
            for (n, (index, start, volume)) in audio_overlays.iter().enumerate() {
                filter.push_str(&format!(
                    ";[{index}:a]atrim=start=0,asetpts=PTS-STARTPTS+{start:.3}/TB,volume={:.3}[a{}]",
                    volume.clamp(0.0, 2.0),
                    n + 1
                ));
                labels.push(format!("[a{}]", n + 1));
            }
            filter.push_str(&format!(
                ";{}amix=inputs={}:duration=first:dropout_transition=2[aout]",
                labels.join(""),
                labels.len()
            ));
            "[aout]".into()
        };
        command.args([
            "-filter_complex",
            &filter,
            "-map",
            &format!("[{current_video}]"),
            "-map",
            &audio_map,
        ]);
        let encoder = self.available_encoder().await;
        command.args(["-c:v", &encoder]);
        if encoder == "libx264" {
            command.args(["-preset", "medium", "-crf", "18"]);
        } else if encoder == "h264_nvenc" {
            command.args(["-preset", "p6", "-cq", "18", "-b:v", "0"]);
        } else {
            command.args(["-b:v", &format!("{}M", preset.video_bitrate_mbps)]);
        }
        command
            .args([
                "-r",
                &preset
                    .max_fps
                    .min(source.fps.round().max(24.0) as u32)
                    .to_string(),
                "-pix_fmt",
                "yuv420p",
                "-colorspace",
                "bt709",
                "-color_primaries",
                "bt709",
                "-color_trc",
                "bt709",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-ar",
                "48000",
                "-movflags",
                "+faststart",
            ])
            .arg(output);
        let result = command.output().await?;
        if !result.status.success() {
            return Err(anyhow!(
                "FFmpeg render: {}",
                tail(&String::from_utf8_lossy(&result.stderr), 18)
            ));
        }
        self.verify(output, preset.width, preset.height).await?;
        Ok(None)
    }

    pub async fn verify(&self, output: &Path, width: u32, height: u32) -> Result<()> {
        let media = self.probe(output).await?;
        if media.width != width || media.height != height {
            bail!("Невірний розмір готового відео")
        }
        let status = Command::new(&self.ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(output)
            .args(["-f", "null", "-"])
            .status()
            .await?;
        if !status.success() {
            bail!("Готове відео містить помилки декодування")
        }
        Ok(())
    }
}

fn discover_binary(variable: &str, name: &str) -> Result<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| find_on_path(name))
        .or_else(|| {
            let root = env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)?
                .join("Microsoft/WinGet/Packages");
            WalkDir::new(root)
                .max_depth(5)
                .into_iter()
                .filter_map(Result::ok)
                .find(|entry| {
                    entry.file_type().is_file()
                        && entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(name)
                })
                .map(|entry| entry.into_path())
        })
        .with_context(|| format!("{name} не знайдено. Встановіть FFmpeg або задайте {variable}"))
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|p| p.join(name))
            .find(|p| p.is_file())
    })
}

fn parse_rate(rate: &str) -> f64 {
    let mut parts = rate.split('/');
    let numerator: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let denominator: f64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

#[derive(Debug)]
struct SceneEvent {
    time: f64,
    score: f32,
    motion: f32,
    saturation: f32,
}

fn parse_signal_events(log: &str) -> Vec<SceneEvent> {
    let mut events = Vec::new();
    let mut pending_time: Option<f64> = None;
    let mut motion: f32 = 0.0;
    let mut saturation: f32 = 0.0;
    for line in log.lines() {
        if let Some(pos) = line.find("pts_time:") {
            if let Some(time) = pending_time.take() {
                let score = (motion / 28.0 * 0.78 + saturation / 70.0 * 0.22).clamp(0.0, 1.0);
                events.push(SceneEvent {
                    time,
                    score,
                    motion,
                    saturation,
                });
            }
            pending_time = line[pos + 9..]
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok());
            motion = 0.0;
            saturation = 0.0;
        }
        if let Some(pos) = line.find("lavfi.signalstats.YDIF=") {
            motion = line[pos + 23..].trim().parse().unwrap_or(0.0);
        }
        if let Some(pos) = line.find("lavfi.signalstats.SATAVG=") {
            saturation = line[pos + 25..].trim().parse().unwrap_or(0.0);
        }
    }
    if let Some(time) = pending_time {
        let score = (motion / 28.0 * 0.78 + saturation / 70.0 * 0.22).clamp(0.0, 1.0);
        events.push(SceneEvent {
            time,
            score,
            motion,
            saturation,
        });
    }
    events
}

fn parse_silence_ranges(log: &str, duration: f64) -> Vec<(f64, f64)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for line in log.lines() {
        if let Some(pos) = line.find("silence_start:") {
            start = line[pos + 14..]
                .trim()
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok());
        }
        if let Some(pos) = line.find("silence_end:") {
            if let (Some(from), Some(to)) = (
                start.take(),
                line[pos + 12..]
                    .trim()
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<f64>().ok()),
            ) {
                ranges.push((from, to));
            }
        }
    }
    if let Some(from) = start {
        ranges.push((from, duration));
    }
    ranges
}

fn kept_segments(edit: &EditDecisionList) -> Vec<(f64, f64)> {
    let duration = (edit.trim_end - edit.trim_start).max(0.0);
    let mut removed: Vec<(f64, f64)> = edit
        .removed_ranges
        .iter()
        .filter_map(|range: &TimeRange| {
            let start = (range.start - edit.trim_start).clamp(0.0, duration);
            let end = (range.end - edit.trim_start).clamp(0.0, duration);
            (end > start).then_some((start, end))
        })
        .collect();
    removed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for range in removed {
        if let Some(last) = merged.last_mut() {
            if range.0 <= last.1 {
                last.1 = last.1.max(range.1);
                continue;
            }
        }
        merged.push(range);
    }
    let mut kept = Vec::new();
    let mut cursor = 0.0;
    for (start, end) in merged {
        if start > cursor {
            kept.push((cursor, start));
        }
        cursor = cursor.max(end);
    }
    if cursor < duration {
        kept.push((cursor, duration));
    }
    kept
}

fn cut_filter(segments: &[(f64, f64)], has_audio: bool) -> (String, String, Option<String>) {
    if segments.len() == 1 && segments[0].0 <= f64::EPSILON {
        return (String::new(), "0:v".into(), has_audio.then(|| "0:a".into()));
    }
    let mut filter = String::new();
    if segments.len() > 1 {
        filter.push_str(&format!("[0:v]split={}", segments.len()));
        for index in 0..segments.len() {
            filter.push_str(&format!("[vsrc{index}]"));
        }
        if has_audio {
            filter.push_str(&format!(";[0:a]asplit={}", segments.len()));
            for index in 0..segments.len() {
                filter.push_str(&format!("[asrc{index}]"));
            }
        }
    }
    for (index, (start, end)) in segments.iter().enumerate() {
        let video_source = if segments.len() > 1 {
            format!("vsrc{index}")
        } else {
            "0:v".into()
        };
        if !filter.is_empty() {
            filter.push(';');
        }
        filter.push_str(&format!(
            "[{video_source}]trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[vcut{index}]"
        ));
        if has_audio {
            let audio_source = if segments.len() > 1 {
                format!("asrc{index}")
            } else {
                "0:a".into()
            };
            filter.push_str(&format!(
                ";[{audio_source}]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[acut{index}]"
            ));
        }
    }
    if segments.len() > 1 {
        filter.push(';');
        for index in 0..segments.len() {
            filter.push_str(&format!("[vcut{index}]"));
            if has_audio {
                filter.push_str(&format!("[acut{index}]"));
            }
        }
        filter.push_str(&format!(
            "concat=n={}:v=1:a={}[vcut]{}",
            segments.len(),
            usize::from(has_audio),
            if has_audio { "[acut]" } else { "" }
        ));
        (filter, "vcut".into(), has_audio.then(|| "acut".into()))
    } else {
        (filter, "vcut0".into(), has_audio.then(|| "acut0".into()))
    }
}

fn base_video_filter(input: &str, mode: &CropMode, zoom: f32, width: u32, height: u32) -> String {
    let z = zoom.clamp(1.0, 3.0);
    match mode {
        CropMode::BlurredBackground => format!("[{input}]split=2[bg][fg];[bg]scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height},boxblur=24:2[blur];[fg]scale={width}:{height}:force_original_aspect_ratio=decrease[front];[blur][front]overlay=(W-w)/2:(H-h)/2,setsar=1[base]"),
        CropMode::Center | CropMode::Dynamic => format!("[{input}]crop='min(iw,ih*9/16/{z})':'min(ih,iw*16/9/{z})':(iw-ow)/2:(ih-oh)/2,scale={width}:{height}:flags=lanczos,setsar=1[base]"),
    }
}

fn audio_map_spec(label: &str) -> String {
    if label.contains(':') {
        format!("{label}?")
    } else {
        format!("[{label}]")
    }
}

fn escape_drawtext(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "’")
        .replace('%', "\\%")
}
fn normalize_color(value: &str) -> String {
    value.trim().trim_start_matches('#').to_string()
}
fn sanitize_label(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(12)
        .collect()
}
fn tail(value: &str, lines: usize) -> String {
    value
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Deserialize)]
struct Probe {
    streams: Vec<ProbeStream>,
    format: ProbeFormat,
}
#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fractional_fps() {
        assert!((parse_rate("60000/1001") - 59.940).abs() < 0.01);
    }

    #[test]
    fn creates_vertical_filter() {
        let filter = base_video_filter("0:v", &CropMode::Center, 1.0, 1080, 1920);
        assert!(filter.contains("scale=1080:1920"));
    }

    #[test]
    fn maps_input_and_filtered_audio_correctly() {
        assert_eq!(audio_map_spec("0:a"), "0:a?");
        assert_eq!(audio_map_spec("acut0"), "[acut0]");
    }

    #[test]
    fn blurred_background_filter_has_two_layers() {
        let filter = base_video_filter("0:v", &CropMode::BlurredBackground, 1.0, 1080, 1920);
        assert!(filter.contains("split=2[bg][fg]"));
        assert!(filter.contains("boxblur=24:2[blur]"));
        assert!(filter.contains("[blur][front]overlay="));
    }

    #[test]
    fn removed_ranges_become_kept_segments() {
        let edit = EditDecisionList {
            project_id: "p".into(),
            candidate_id: "c".into(),
            trim_start: 10.0,
            trim_end: 20.0,
            removed_ranges: vec![TimeRange {
                start: 13.0,
                end: 15.0,
            }],
            crop: banshee_domain::CropSettings {
                mode: CropMode::Center,
                zoom: 1.0,
                center_x: 0.5,
                center_y: 0.5,
            },
            overlays: vec![],
            revision: 1,
        };
        assert_eq!(kept_segments(&edit), vec![(0.0, 3.0), (5.0, 10.0)]);
    }

    #[test]
    fn scene_log_is_parsed() {
        let events = parse_signal_events("x frame:0 pts_time:12.5 pos:0\nlavfi.signalstats.SATAVG=20\nlavfi.signalstats.YDIF=18\n");
        assert_eq!(events.len(), 1);
        assert!((events[0].time - 12.5).abs() < 0.01);
    }

    #[test]
    fn parses_silence_intervals() {
        let ranges = parse_silence_ranges("silence_start: 3.2\nsilence_end: 8.4 | x\n", 20.0);
        assert_eq!(ranges, vec![(3.2, 8.4)]);
    }
}
