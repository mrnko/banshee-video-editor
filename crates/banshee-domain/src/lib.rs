use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMedia {
    pub path: String,
    pub file_name: String,
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub bitrate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source: SourceMedia,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Imported,
    Analyzing,
    Ready,
    Rendering,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSettings {
    pub platform: SocialPlatform,
    pub moment_count: Option<u8>,
    pub min_duration_seconds: u16,
    pub max_duration_seconds: u16,
    pub model: String,
    pub use_openai: bool,
    pub prompt: String,
}

impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            platform: SocialPlatform::TikTok,
            moment_count: None,
            min_duration_seconds: 15,
            max_duration_seconds: 45,
            model: "gpt-5.6-terra".into(),
            use_openai: true,
            prompt: DEFAULT_HIGHLIGHTS_PROMPT.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SocialPlatform {
    TikTok,
    Reels,
    Shorts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateClip {
    pub id: String,
    pub project_id: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub score: f32,
    pub reason: String,
    pub thumbnail_path: Option<String>,
    pub estimated_cost_usd: f64,
    pub analysis_source: AnalysisSource,
}

impl CandidateClip {
    pub fn duration_seconds(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisSource {
    Local,
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditDecisionList {
    pub project_id: String,
    pub candidate_id: String,
    pub trim_start: f64,
    pub trim_end: f64,
    pub removed_ranges: Vec<TimeRange>,
    pub crop: CropSettings,
    pub overlays: Vec<Overlay>,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropSettings {
    pub mode: CropMode,
    pub zoom: f32,
    pub center_x: f32,
    pub center_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CropMode {
    Dynamic,
    Center,
    BlurredBackground,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Overlay {
    Text {
        id: String,
        text: String,
        color: String,
        size: u16,
        x: f32,
        y: f32,
        start: f64,
        end: f64,
    },
    Audio {
        id: String,
        asset_id: String,
        start: f64,
        end: f64,
        volume: f32,
    },
    Video {
        id: String,
        asset_id: String,
        start: f64,
        end: f64,
        x: f32,
        y: f32,
        scale: f32,
        chroma_key: Option<ChromaKey>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromaKey {
    pub color: String,
    pub similarity: f32,
    pub blend: f32,
    pub spill: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPreset {
    pub id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub max_fps: u32,
    pub video_bitrate_mbps: u16,
    pub ai_upscale: UpscaleMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpscaleMode {
    Off,
    Auto,
    Anime,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderJob {
    pub id: String,
    pub project_id: String,
    pub candidate_id: String,
    pub output_path: String,
    pub status: JobStatus,
    pub progress: f32,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
    pub id: String,
    pub project_id: Option<String>,
    pub candidate_id: Option<String>,
    pub model: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub audio_seconds: f64,
    pub estimated_cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub path: String,
    pub kind: AssetKind,
    pub missing: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFolder {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Video,
    Audio,
    Image,
}

pub const DEFAULT_HIGHLIGHTS_PROMPT: &str = r#"Ти — монтажний асистент коротких вертикальних відео. Обирай лише найдинамічніші, візуально насичені й зрозумілі без контексту моменти. Оціни дію, несподіваність, емоційність, придатність для TikTok/Reels/Shorts та силу hook у перші 1–2 секунди. Не пропонуй довгі вступи, меню, завантаження, очікування, повтори або статичні сцени. Побудуй кожен кліп як hook → розвиток → кульмінація → коротке завершення. Поверни точні таймкоди, оцінку 0–100 і стислу причину вибору. Не вигадуй подій, яких немає на наданих кадрах і в транскрипції."#;

pub fn default_presets() -> Vec<RenderPreset> {
    vec![
        RenderPreset {
            id: "balanced-720".into(),
            label: "720p Balanced".into(),
            width: 720,
            height: 1280,
            max_fps: 60,
            video_bitrate_mbps: 8,
            ai_upscale: UpscaleMode::Off,
        },
        RenderPreset {
            id: "high-1080".into(),
            label: "1080p High".into(),
            width: 1080,
            height: 1920,
            max_fps: 60,
            video_bitrate_mbps: 16,
            ai_upscale: UpscaleMode::Auto,
        },
        RenderPreset {
            id: "source-max".into(),
            label: "Source-aware Maximum".into(),
            width: 1080,
            height: 1920,
            max_fps: 60,
            video_bitrate_mbps: 24,
            ai_upscale: UpscaleMode::Auto,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_duration_never_negative() {
        let clip = CandidateClip {
            id: "1".into(),
            project_id: "p".into(),
            start_seconds: 20.0,
            end_seconds: 10.0,
            score: 0.0,
            reason: String::new(),
            thumbnail_path: None,
            estimated_cost_usd: 0.0,
            analysis_source: AnalysisSource::Local,
        };
        assert_eq!(clip.duration_seconds(), 0.0);
    }

    #[test]
    fn high_preset_is_present() {
        assert!(default_presets()
            .iter()
            .any(|p| p.id == "high-1080" && p.width == 1080));
    }
}
