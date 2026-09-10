import { create } from "zustand";
import type { Asset, AssetFolder, BootstrapData, CandidateClip, Page, Project, UpdaterState } from "../lib/types";

interface AppStore {
  page: Page;
  data?: BootstrapData;
  currentProject?: Project;
  candidates: CandidateClip[];
  busy: boolean;
  toast?: string;
  updater: UpdaterState;
  setPage: (page: Page) => void;
  setData: (data: BootstrapData) => void;
  setProject: (project?: Project) => void;
  addProject: (project: Project) => void;
  replaceProject: (project: Project) => void;
  removeProject: (projectId: string) => void;
  setCandidates: (candidates: CandidateClip[]) => void;
  addAsset: (asset: Asset) => void;
  replaceAsset: (asset: Asset) => void;
  removeAsset: (assetId: string) => void;
  addAssetFolder: (folder: AssetFolder) => void;
  setBusy: (busy: boolean) => void;
  notify: (message: string) => void;
  clearToast: () => void;
  setUpdater: (patch: Partial<UpdaterState>) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  page: "dashboard", candidates: [], busy: false, updater: { status: "idle", downloadedBytes: 0, modalOpen: false },
  setPage: page => set({ page }),
  setData: data => set({ data }),
  setProject: currentProject => set({ currentProject, page: currentProject ? "projects" : "dashboard" }),
  addProject: project => set(state => state.data ? ({ data: { ...state.data, projects: [project, ...state.data.projects] }, currentProject: project, page: "projects" }) : state),
  replaceProject: project => set(state => state.data ? ({ data: { ...state.data, projects: state.data.projects.map(item => item.id === project.id ? project : item) }, currentProject: state.currentProject?.id === project.id ? project : state.currentProject }) : state),
  removeProject: projectId => set(state => state.data ? ({ data: { ...state.data, projects: state.data.projects.filter(item => item.id !== projectId) }, currentProject: state.currentProject?.id === projectId ? undefined : state.currentProject, page: state.currentProject?.id === projectId ? "projects" : state.page }) : state),
  setCandidates: candidates => set({ candidates }),
  addAsset: asset => set(state => state.data ? ({ data: { ...state.data, assets: [asset, ...state.data.assets] } }) : state),
  replaceAsset: asset => set(state => state.data ? ({ data: { ...state.data, assets: state.data.assets.map(item => item.id === asset.id ? asset : item) } }) : state),
  removeAsset: assetId => set(state => state.data ? ({ data: { ...state.data, assets: state.data.assets.filter(item => item.id !== assetId) } }) : state),
  addAssetFolder: folder => set(state => state.data ? ({ data: { ...state.data, assetFolders: [...state.data.assetFolders, folder] } }) : state),
  setBusy: busy => set({ busy }),
  notify: toast => set({ toast }),
  clearToast: () => set({ toast: undefined }),
  setUpdater: patch => set(state => ({ updater: { ...state.updater, ...patch } }))
}));
