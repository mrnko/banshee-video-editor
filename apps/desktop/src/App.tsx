import { useEffect, useState } from "react";
import { AlertCircle, LoaderCircle, X } from "lucide-react";
import { Sidebar } from "./components/Sidebar";
import { UpdateCenter } from "./components/UpdateCenter";
import { Dashboard } from "./features/dashboard/Dashboard";
import { Library } from "./features/library/Library";
import { Projects } from "./features/projects/ProjectWorkspace";
import { Releases } from "./features/releases/Releases";
import { Settings } from "./features/settings/Settings";
import { api } from "./lib/api";
import { useAppStore } from "./state/useAppStore";
import mark from "./assets/banshee-mark.svg";
import { checkForUpdates, lastUpdateCheck, UPDATE_CHECK_INTERVAL_MS } from "./lib/updater";

export default function App() {
  const { page, data, setData, toast, clearToast, notify, setUpdater } = useAppStore();
  const [loadingStage, setLoadingStage] = useState("Підключаємо локальне сховище…");
  useEffect(() => { setLoadingStage("Завантажуємо проєкти та медіатеку…"); api.bootstrap().then(value => { setLoadingStage("Готово"); setData(value); const last = lastUpdateCheck(); if (last) setUpdater({ lastCheckedAt: last }); if (!last || Date.now() - new Date(last).getTime() >= UPDATE_CHECK_INTERVAL_MS) void checkForUpdates(false); }).catch(error => notify(String(error))); }, [notify, setData, setUpdater]);
  useEffect(() => { if (!data) return; const timer = window.setInterval(() => void checkForUpdates(false), UPDATE_CHECK_INTERVAL_MS); return () => window.clearInterval(timer); }, [data]);
  useEffect(() => { if (!toast) return; const timer = setTimeout(clearToast, 4500); return () => clearTimeout(timer); }, [toast, clearToast]);
  if (!data) return <div className="splash"><div className="splash-mark"><img src={mark} alt="Banshee"/><LoaderCircle className="spin splash-spinner"/></div><strong>Banshee</strong><span>{loadingStage}</span></div>;
  return <div className="app-shell"><Sidebar/><div className="content">{page === "dashboard" && <Dashboard/>}{page === "projects" && <Projects/>}{page === "library" && <Library/>}{page === "settings" && <Settings/>}{page === "releases" && <Releases/>}</div><UpdateCenter/>{toast&&<div className="toast"><AlertCircle size={18}/><span>{toast}</span><button onClick={clearToast}><X size={15}/></button></div>}</div>;
}
