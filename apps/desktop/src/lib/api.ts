import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AnalysisSettings, AppSettings, Asset, AssetFolder, BootstrapData, CandidateClip, EditDecisionList, ModelOption, Project, RenderJob } from "./types";
import { DEFAULT_PROMPT } from "./types";

const now = new Date().toISOString();
const demoProject: Project = {
  id: "demo", name: "Mobile Legends — Ranked Match", status: "ready", createdAt: now, updatedAt: now,
  source: { path: "D:\\Projects\\dana-video\\IMG_4574.MP4", fileName: "IMG_4574.MP4", durationSeconds: 1118.76, width: 1920, height: 884, fps: 59.95, videoCodec: "h264", audioCodec: "aac", bitrate: 4_007_306 }
};
const demoCandidates: CandidateClip[] = [
  { id: "c1", projectId: "demo", startSeconds: 683.5, endSeconds: 708, score: 96, reason: "Сильний hook з усуненням суперника, статус «Легендарно» та ще одна кульмінація у фіналі.", estimatedCostUsd: 0.0038, analysisSource: "openAi" },
  { id: "c2", projectId: "demo", startSeconds: 743.5, endSeconds: 761, score: 92, reason: "Швидка атака з кущів, серія яскравих влучань і подвійне вбивство.", estimatedCostUsd: 0.0032, analysisSource: "openAi" },
  { id: "c3", projectId: "demo", startSeconds: 891.5, endSeconds: 907, score: 89, reason: "Командний бій починається одразу, має насичені ефекти та зрозумілий фінал.", estimatedCostUsd: 0.0029, analysisSource: "openAi" }
];
const demoBootstrap: BootstrapData = {
  projects: [demoProject], assets: [], assetFolders: [], settings: { autosaveSeconds: 15, model: "gpt-5.6-terra", adminCostSync: false, prompt: DEFAULT_PROMPT },
  stats: { spendToday: 0.0427, spendPoints: [{ label: "03.09", value: .006 }, { label: "04.09", value: .012 }, { label: "05.09", value: .009 }, { label: "06.09", value: .021 }, { label: "07.09", value: .018 }, { label: "08.09", value: .031 }, { label: "09.09", value: .0427 }], videosToday: 5, videosYesterday: 3, videosWeek: 19, videosMonth: 47 },
  presets: [
    { id: "balanced-720", label: "720p Balanced", width: 720, height: 1280, maxFps: 60, videoBitrateMbps: 8, aiUpscale: "off" },
    { id: "high-1080", label: "1080p High", width: 1080, height: 1920, maxFps: 60, videoBitrateMbps: 16, aiUpscale: "auto" },
    { id: "source-max", label: "Source-aware Maximum", width: 1080, height: 1920, maxFps: 60, videoBitrateMbps: 24, aiUpscale: "auto" }
  ], ffmpegAvailable: true, upscalerAvailable: false, apiKeyConfigured: false, version: "0.1.0"
};

const native = isTauri();
const sleep = (ms = 250) => new Promise(resolve => setTimeout(resolve, ms));

export const api = {
  native,
  fileUrl(path?: string) { return path && native ? convertFileSrc(path) : undefined; },
  async bootstrap(): Promise<BootstrapData> { if (!native) { await sleep(); return demoBootstrap; } return invoke("bootstrap"); },
  async chooseVideo(): Promise<string | null> {
    if (!native) return demoProject.source.path;
    const result = await open({ multiple: false, filters: [{ name: "Відео", extensions: ["mp4", "mov", "mkv", "webm", "avi"] }] });
    return typeof result === "string" ? result : null;
  },
  async importVideo(path: string): Promise<Project> { if (!native) { await sleep(500); return demoProject; } return invoke("import_video", { path }); },
  async candidates(projectId: string): Promise<CandidateClip[]> { if (!native) return projectId === "demo" ? demoCandidates : []; return invoke("project_candidates", { projectId }); },
  async analyze(projectId: string, settings: AnalysisSettings): Promise<CandidateClip[]> { if (!native) { await sleep(900); return demoCandidates.map(c => ({ ...c, projectId })); } return invoke("analyze_project", { projectId, settings }); },
  async saveEdit(edit: EditDecisionList) { if (!native) return; return invoke("save_edit", { edit }); },
  async loadEdit(candidateId: string): Promise<EditDecisionList | null> { if (!native) return null; return invoke("load_edit", { candidateId }); },
  async render(projectId: string, candidateId: string, presetId: string): Promise<RenderJob> {
    if (!native) { await sleep(1000); return { id: crypto.randomUUID(), projectId, candidateId, outputPath: "D:\\Exports\\banshee-demo.mp4", status: "completed", progress: 100 }; }
    const outputPath = await save({ defaultPath: `banshee-${candidateId.slice(0, 8)}.mp4`, filters: [{ name: "MP4", extensions: ["mp4"] }] });
    if (!outputPath) throw new Error("RENDER_CANCELLED");
    return invoke("render_clip", { projectId, candidateId, presetId, outputPath });
  },
  async chooseAsset(): Promise<string | null> { if (!native) return null; const result = await open({ multiple: false }); return typeof result === "string" ? result : null; },
  async importAsset(path: string, folderId?: string): Promise<Asset> { return invoke("import_asset", { path, folderId }); },
  async createAssetFolder(name: string): Promise<AssetFolder> { if (!native) return { id: crypto.randomUUID(), name, createdAt: new Date().toISOString() }; return invoke("create_asset_folder", { name }); },
  async saveSettings(settings: AppSettings) { if (!native) return; return invoke("save_settings", { settings }); },
  async setApiKey(kind: "openai" | "admin", value: string) { if (!native) return; return invoke("set_api_key", { kind, value }); },
  async models(): Promise<ModelOption[]> { if (!native) return ["gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"].map(id => ({ id, available: id === "gpt-5.6-terra", experimental: false, recommended: id === "gpt-5.6-terra" })); return invoke("list_openai_models"); },
  async syncCosts(): Promise<number> { if (!native) return 1.284; return invoke("sync_openai_costs"); },
  async reveal(path: string) { if (!native) return; return invoke("reveal_file", { path }); },
  async share(path: string) { if (!native) return; return invoke("share_file", { path }); }
};
