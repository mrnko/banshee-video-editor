import { create } from "zustand";
import type { Asset, AssetFolder, BootstrapData, CandidateClip, Page, Project } from "../lib/types";

interface AppStore {
  page: Page;
  data?: BootstrapData;
  currentProject?: Project;
  candidates: CandidateClip[];
  busy: boolean;
  toast?: string;
  setPage: (page: Page) => void;
  setData: (data: BootstrapData) => void;
  setProject: (project?: Project) => void;
  addProject: (project: Project) => void;
  setCandidates: (candidates: CandidateClip[]) => void;
  addAsset: (asset: Asset) => void;
  addAssetFolder: (folder: AssetFolder) => void;
  setBusy: (busy: boolean) => void;
  notify: (message: string) => void;
  clearToast: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  page: "dashboard", candidates: [], busy: false,
  setPage: page => set({ page }),
  setData: data => set({ data }),
  setProject: currentProject => set({ currentProject, page: currentProject ? "projects" : "dashboard" }),
  addProject: project => set(state => state.data ? ({ data: { ...state.data, projects: [project, ...state.data.projects] }, currentProject: project, page: "projects" }) : state),
  setCandidates: candidates => set({ candidates }),
  addAsset: asset => set(state => state.data ? ({ data: { ...state.data, assets: [asset, ...state.data.assets] } }) : state),
  addAssetFolder: folder => set(state => state.data ? ({ data: { ...state.data, assetFolders: [...state.data.assetFolders, folder] } }) : state),
  setBusy: busy => set({ busy }),
  notify: toast => set({ toast }),
  clearToast: () => set({ toast: undefined })
}));
