import { BarChart3, FolderKanban, LibraryBig, Settings, Sparkles } from "lucide-react";
import type { Page } from "../lib/types";
import { useAppStore } from "../state/useAppStore";
import { Logo } from "./Logo";

const entries: Array<{ id: Page; label: string; icon: typeof BarChart3 }> = [
  { id: "dashboard", label: "Огляд", icon: BarChart3 },
  { id: "projects", label: "Проєкти", icon: FolderKanban },
  { id: "library", label: "Медіатека", icon: LibraryBig },
  { id: "settings", label: "Налаштування", icon: Settings }
];

export function Sidebar() {
  const { page, setPage, data } = useAppStore();
  return <aside className="sidebar">
    <Logo />
    <nav>{entries.map(({ id, label, icon: Icon }) => <button key={id} className={page === id ? "active" : ""} onClick={() => setPage(id)}><Icon size={18} />{label}</button>)}</nav>
    <div className="sidebar-spacer" />
    <div className="pro-tip"><Sparkles size={18}/><div><strong>Local first</strong><span>Ваше відео залишається на ПК</span></div></div>
    <button className="version-chip" onClick={() => setPage("releases")}>Banshee v{data?.version ?? "0.1.0"}</button>
  </aside>;
}

