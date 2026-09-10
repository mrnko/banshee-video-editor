import { ArrowRight, Clock3, Film, Plus, ShieldCheck, Sparkles, UploadCloud, WandSparkles } from "lucide-react";
import { SpendChart } from "../../components/SpendChart";
import { api } from "../../lib/api";
import { formatMoney, formatTime } from "../../lib/format";
import { useAppStore } from "../../state/useAppStore";

export function Dashboard() {
  const { data, addProject, setProject, setBusy, busy, notify } = useAppStore();
  if (!data) return null;
  async function importVideo() {
    try {
      const path = await api.chooseVideo(); if (!path) return;
      setBusy(true); const project = await api.importVideo(path); addProject(project);
    } catch (error) { notify(String(error)); } finally { setBusy(false); }
  }
  const stats = data.stats;
  return <main className="page dashboard-page">
    <header className="page-header"><div><span className="eyebrow">Творча студія</span><h1>Перетворіть довге відео<br/>на моменти, які хочеться додивитися.</h1><p>Локальний аналіз знаходить динаміку. AI допомагає зрозуміти, що справді варте публікації.</p></div><button className="primary" onClick={importVideo} disabled={busy}><Plus size={18}/>{busy ? "Імпортуємо…" : "Новий проєкт"}</button></header>
    <section className="drop-card" onClick={importVideo} onDragOver={e => e.preventDefault()}>
      <div className="drop-icon"><UploadCloud size={26}/></div><div><strong>Перетягніть відео сюди</strong><span>MP4, MOV, MKV, WebM або AVI · файл не залишає ваш комп’ютер</span></div><ArrowRight size={20}/>
    </section>
    <section className="stats-grid">
      <Metric label="Відео сьогодні" value={stats.videosToday} note={`Учора ${stats.videosYesterday}`} icon={Film}/>
      <Metric label="За останні 7 днів" value={stats.videosWeek} note="готових кліпів" icon={Sparkles}/>
      <Metric label="За останні 30 днів" value={stats.videosMonth} note="успішних рендерів" icon={WandSparkles}/>
      <Metric label="Витрати сьогодні" value={formatMoney(stats.spendToday)} note="локальна оцінка" icon={ShieldCheck}/>
    </section>
    <section className="dashboard-grid">
      <article className="panel spend-panel"><div className="panel-heading"><div><span className="eyebrow">OpenAI API</span><h2>Динаміка витрат</h2></div><div className="total"><span>7 днів</span><strong>{formatMoney(stats.spendPoints.reduce((s,p)=>s+p.value,0))}</strong></div></div><SpendChart points={stats.spendPoints}/></article>
      <article className="panel recent-panel"><div className="panel-heading"><div><span className="eyebrow">Продовжити</span><h2>Останні проєкти</h2></div></div>
        <div className="recent-list">{data.projects.slice(0,4).map(project => <button key={project.id} onClick={() => setProject(project)}><div className="project-thumb"><Film size={20}/></div><div><strong>{project.name}</strong><span><Clock3 size={13}/>{formatTime(project.source.durationSeconds)} · {project.source.width}×{project.source.height}</span></div><span className={`status ${project.status}`}>{statusLabel(project.status)}</span></button>)}{!data.projects.length && <div className="empty-small">Ще немає проєктів</div>}</div>
      </article>
    </section>
  </main>;
}

function Metric({ label, value, note, icon: Icon }: { label: string; value: string | number; note: string; icon: typeof Film }) {
  return <article className="metric"><div className="metric-icon"><Icon size={19}/></div><span>{label}</span><strong>{value}</strong><small>{note}</small></article>;
}
function statusLabel(value: string) { return ({ imported: "Новий", analyzing: "Аналіз", ready: "Готовий", rendering: "Рендер", completed: "Завершено", failed: "Помилка" } as Record<string,string>)[value] ?? value; }

