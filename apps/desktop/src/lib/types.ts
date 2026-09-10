export type Page = "dashboard" | "projects" | "library" | "settings" | "releases";
export type ProjectStatus = "imported" | "analyzing" | "ready" | "rendering" | "completed" | "failed";

export interface SourceMedia {
  path: string; fileName: string; durationSeconds: number; width: number; height: number;
  fps: number; videoCodec: string; audioCodec?: string; bitrate: number;
}
export interface Project { id: string; name: string; source: SourceMedia; createdAt: string; updatedAt: string; status: ProjectStatus }
export interface CandidateClip {
  id: string; projectId: string; startSeconds: number; endSeconds: number; score: number;
  reason: string; thumbnailPath?: string; estimatedCostUsd: number; analysisSource: "local" | "openAi";
}
export interface AnalysisSettings {
  platform: "tikTok" | "reels" | "shorts"; momentCount?: number;
  minDurationSeconds: number; maxDurationSeconds: number; model: string; useOpenai: boolean; prompt: string;
}
export interface CropSettings { mode: "dynamic" | "center" | "blurredBackground"; zoom: number; centerX: number; centerY: number }
export type Overlay =
  | { type: "text"; id: string; text: string; color: string; size: number; x: number; y: number; start: number; end: number }
  | { type: "audio"; id: string; assetId: string; start: number; end: number; volume: number }
  | { type: "video"; id: string; assetId: string; start: number; end: number; x: number; y: number; scale: number; chromaKey?: ChromaKey };
export interface ChromaKey { color: string; similarity: number; blend: number; spill: number }
export interface EditDecisionList {
  projectId: string; candidateId: string; trimStart: number; trimEnd: number;
  removedRanges: Array<{ start: number; end: number }>; crop: CropSettings; overlays: Overlay[]; revision: number;
}
export interface RenderPreset { id: string; label: string; width: number; height: number; maxFps: number; videoBitrateMbps: number; aiUpscale: "off" | "auto" | "anime" | "general" }
export interface RenderJob { id: string; projectId: string; candidateId: string; outputPath: string; status: string; progress: number; warning?: string }
export interface Asset { id: string; folderId?: string; name: string; path: string; kind: "video" | "audio" | "image"; missing: boolean; createdAt: string }
export interface AssetFolder { id: string; name: string; createdAt: string }
export interface AppSettings { autosaveSeconds: number; model: string; adminCostSync: boolean; prompt: string }
export interface SpendPoint { label: string; value: number }
export interface DashboardStats { spendToday: number; spendPoints: SpendPoint[]; videosToday: number; videosYesterday: number; videosWeek: number; videosMonth: number }
export interface ModelOption { id: string; available: boolean; experimental: boolean; recommended: boolean }
export interface BootstrapData {
  projects: Project[]; assets: Asset[]; assetFolders: AssetFolder[]; settings: AppSettings; stats: DashboardStats;
  presets: RenderPreset[]; ffmpegAvailable: boolean; upscalerAvailable: boolean; apiKeyConfigured: boolean; version: string;
}

export const DEFAULT_PROMPT = "Ти — монтажний асистент коротких вертикальних відео. Обирай лише найдинамічніші, візуально насичені й зрозумілі без контексту моменти. Оціни дію, несподіваність, емоційність, придатність для TikTok/Reels/Shorts та силу hook у перші 1–2 секунди. Не пропонуй довгі вступи, меню, завантаження, очікування, повтори або статичні сцени. Побудуй кожен кліп як hook → розвиток → кульмінація → коротке завершення. Поверни точні таймкоди, оцінку 0–100 і стислу причину вибору. Не вигадуй подій, яких немає на наданих кадрах і в транскрипції.";
