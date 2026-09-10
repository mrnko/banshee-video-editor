import { useEffect } from "react";
import { AlertCircle, LoaderCircle, X } from "lucide-react";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./features/dashboard/Dashboard";
import { Library } from "./features/library/Library";
import { Projects } from "./features/projects/ProjectWorkspace";
import { Releases } from "./features/releases/Releases";
import { Settings } from "./features/settings/Settings";
import { api } from "./lib/api";
import { useAppStore } from "./state/useAppStore";

export default function App() {
  const { page, data, setData, toast, clearToast, notify } = useAppStore();
  useEffect(() => { api.bootstrap().then(setData).catch(error => notify(String(error))); }, [notify, setData]);
  useEffect(() => { if (!toast) return; const timer = setTimeout(clearToast, 4500); return () => clearTimeout(timer); }, [toast, clearToast]);
  if (!data) return <div className="splash"><div className="splash-mark"><LoaderCircle className="spin"/></div><strong>Banshee</strong><span>Готуємо студію…</span></div>;
  return <div className="app-shell"><Sidebar/><div className="content">{page === "dashboard" && <Dashboard/>}{page === "projects" && <Projects/>}{page === "library" && <Library/>}{page === "settings" && <Settings/>}{page === "releases" && <Releases/>}</div>{toast&&<div className="toast"><AlertCircle size={18}/><span>{toast}</span><button onClick={clearToast}><X size={15}/></button></div>}</div>;
}
