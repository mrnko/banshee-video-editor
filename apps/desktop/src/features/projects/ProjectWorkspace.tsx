import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft, Bot, Check, ChevronDown, Clock3, Film, Gauge, Image, Pencil, Play, RefreshCw, Settings2, Sparkles, Trash2, WandSparkles, X } from "lucide-react";
import { api } from "../../lib/api";
import { formatMoney, formatTime } from "../../lib/format";
import { DEFAULT_PROMPT, type AnalysisSettings, type CandidateClip } from "../../lib/types";
import { useAppStore } from "../../state/useAppStore";
import { ClipEditor } from "../editor/ClipEditor";

export function Projects() {
  const { data, currentProject, setProject, replaceProject, removeProject, notify } = useAppStore();
  const [menu, setMenu] = useState<{ id: string; name: string; x: number; y: number }>();
  if (!data) return null;
  if (currentProject) return <ProjectWorkspace />;
  async function rename() { if (!menu) return; const name = window.prompt("Нова назва проєкту", menu.name); if (!name?.trim()) return; try { replaceProject(await api.renameProject(menu.id, name)); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  async function remove() { if (!menu || !window.confirm(`Видалити «${menu.name}» з Banshee? Вихідний файл залишиться на диску.`)) return; try { await api.deleteProject(menu.id); removeProject(menu.id); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  return <main className="page" onClick={() => setMenu(undefined)}><header className="page-header compact"><div><span className="eyebrow">Ваші матеріали</span><h1>Проєкти</h1><p>Імпортовані відео та чернетки монтажу.</p></div></header><div className="project-grid">{data.projects.map(project => <button className="project-card" key={project.id} onClick={() => setProject(project)} onContextMenu={event => { event.preventDefault(); setMenu({ id: project.id, name: project.name, x: event.clientX, y: event.clientY }); }}><ProjectThumbnail project={project}/><div className="project-card-body"><strong>{project.name}</strong><span>{formatTime(project.source.durationSeconds)} · {project.source.fps.toFixed(1)} FPS</span></div></button>)}</div>{menu && <ContextMenu x={menu.x} y={menu.y} onRename={rename} onDelete={remove}/>}</main>;
}

function ProjectThumbnail({ project }: { project: import("../../lib/types").Project }) {
  const thumbnail = api.fileUrl(project.thumbnailPath); const source = api.fileUrl(project.source.path);
  return <div className="project-preview">{thumbnail ? <img src={thumbnail} alt=""/> : source ? <video src={source} muted playsInline preload="metadata" onLoadedMetadata={event => { event.currentTarget.currentTime = .1; }}/> : <Film size={32}/>}<span>{project.source.width}×{project.source.height}</span></div>;
}

function ProjectWorkspace() {
  const { currentProject: project, candidates, setCandidates, setProject, busy, setBusy, data, notify } = useAppStore();
  const [editorClip, setEditorClip] = useState<CandidateClip>();
  const [previewClip, setPreviewClip] = useState<CandidateClip>();
  const [autoCount, setAutoCount] = useState(true);
  const [count, setCount] = useState(5);
  const [model, setModel] = useState(data?.settings.model ?? "gpt-5.6-terra");
  const [preset, setPreset] = useState("high-1080");
  const [platform, setPlatform] = useState<AnalysisSettings["platform"]>("tikTok");
  const [menu, setMenu] = useState<{ id: string; name: string; x: number; y: number }>();
  useEffect(() => { if (project) api.candidates(project.id).then(setCandidates).catch(() => setCandidates([])); }, [project?.id, setCandidates]);
  const duration = useMemo(() => project?.source.durationSeconds ?? 0, [project]);
  if (!project || !data) return null;
  const activeProject = project;
  const bootstrap = data;
  async function analyze() {
    setBusy(true);
    try {
      const result = await api.analyze(activeProject.id, { platform, momentCount: autoCount ? undefined : count, minDurationSeconds: 15, maxDurationSeconds: 45, model, useOpenai: bootstrap.apiKeyConfigured, prompt: bootstrap.settings.prompt || DEFAULT_PROMPT });
      setCandidates(result); notify(`Знайдено ${result.length} сильних моментів`);
    } catch (error) { notify(String(error)); } finally { setBusy(false); }
  }
  async function renameMoment() { if (!menu) return; const name = window.prompt("Нова назва моменту", menu.name); if (!name?.trim()) return; try { const updated = await api.renameCandidate(menu.id, name); setCandidates(candidates.map(item => item.id === updated.id ? updated : item)); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  async function deleteMoment() { if (!menu || !window.confirm(`Видалити «${menu.name}» з Banshee?`)) return; try { await api.deleteCandidate(menu.id); setCandidates(candidates.filter(item => item.id !== menu.id)); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  return <main className="page workspace-page" onClick={() => setMenu(undefined)}>
    <header className="workspace-header"><button className="icon-button" onClick={() => setProject(undefined)}><ArrowLeft size={19}/></button><div><span className="eyebrow">Проєкт</span><h1>{project.name}</h1></div><div className="source-pill"><Film size={16}/><span>{project.source.width}×{project.source.height}</span><span>{project.source.fps.toFixed(1)} FPS</span><span>{formatTime(duration)}</span></div></header>
    <section className="analysis-bar">
      <Field icon={Image} label="Формат"><select value={platform} onChange={e => setPlatform(e.target.value as AnalysisSettings["platform"])}><option value="tikTok">TikTok 9:16</option><option value="reels">Reels 9:16</option><option value="shorts">Shorts 9:16</option></select></Field>
      <Field icon={Sparkles} label="Кількість"><button className="select-like" onClick={() => setAutoCount(!autoCount)}>{autoCount ? "Автоматично" : `${count} моментів`}<ChevronDown size={15}/></button>{!autoCount && <input className="count-input" type="number" min={1} max={20} value={count} onChange={e=>setCount(Math.min(20,Math.max(1,+e.target.value)))}/>}</Field>
      <Field icon={Bot} label="Модель"><select value={model} onChange={e => setModel(e.target.value)}><option>gpt-5.6-terra</option><option>gpt-6-astra</option><option>gpt-5.6-sol</option><option>gpt-5.6-luna</option></select></Field>
      <Field icon={Gauge} label="Якість"><select value={preset} onChange={e => setPreset(e.target.value)}>{data.presets.map(p => <option key={p.id} value={p.id}>{p.label}</option>)}</select></Field>
      <button className="primary analyze-button" onClick={analyze} disabled={busy}>{busy ? <RefreshCw size={18} className="spin"/> : <WandSparkles size={18}/>} {busy ? "Аналізуємо…" : candidates.length ? "Повторити" : "Знайти моменти"}</button>
    </section>
    {!data.ffmpegAvailable && <div className="warning-banner">FFmpeg не знайдено. Вкажіть шлях у `.env` або встановіть через WinGet.</div>}
    <section className="section-heading"><div><span className="eyebrow">Результати</span><h2>{candidates.length ? `${candidates.length} моментів готові до перегляду` : "Почніть локальний аналіз"}</h2></div>{candidates.length > 0 && <span className="privacy-badge"><Check size={14}/> До API надіслано лише preview-кадри</span>}</section>
    {candidates.length ? <div className="clips-grid">{candidates.map((clip, index) => <article className="clip-card" key={clip.id} onContextMenu={event => { event.preventDefault(); setMenu({ id: clip.id, name: clip.name ?? `Момент #${index + 1}`, x: event.clientX, y: event.clientY }); }}>
      <button className="clip-visual" onClick={() => setPreviewClip(clip)} aria-label={`Переглянути момент ${index + 1}`}><ClipThumbnail clip={clip} index={index}/><span className="play"><Play fill="currentColor" size={20}/></span><span className="duration">{formatTime(clip.endSeconds-clip.startSeconds)}</span><span className="score">{Math.round(clip.score)}</span></button>
      <div className="clip-body"><div className="clip-meta"><span><Clock3 size={13}/>{formatTime(clip.startSeconds)} — {formatTime(clip.endSeconds)}</span><span className={clip.analysisSource === "openAi" ? "ai-label" : "local-label"}>{clip.analysisSource === "openAi" ? "AI" : "LOCAL"}</span></div><h3>{clip.name ?? `Момент #${index + 1}`}</h3><p>{clip.reason}</p><div className="clip-footer"><span>≈ {formatMoney(clip.estimatedCostUsd)}</span><button onClick={() => setEditorClip(clip)}><Pencil size={15}/>Редагувати</button></div></div>
    </article>)}</div> : <div className="empty-state"><div><Settings2 size={28}/></div><h3>Все готово до старту</h3><p>Налаштуйте кількість кліпів і натисніть «Знайти моменти».</p></div>}
    {previewClip && <ClipPreview clip={previewClip} sourcePath={project.source.path} onClose={() => setPreviewClip(undefined)} onEdit={() => { setEditorClip(previewClip); setPreviewClip(undefined); }}/>}
    {editorClip && <ClipEditor clip={editorClip} project={project} presetId={preset} onClose={() => setEditorClip(undefined)}/>}
    {menu && <ContextMenu x={menu.x} y={menu.y} onRename={renameMoment} onDelete={deleteMoment}/>}
  </main>;
}

function ContextMenu({ x, y, onRename, onDelete }: { x: number; y: number; onRename: () => void; onDelete: () => void }) {
  return <div className="context-menu" style={{ left: x, top: y }} onClick={event => event.stopPropagation()}><button onClick={onRename}><Pencil size={14}/>Перейменувати</button><button className="danger" onClick={onDelete}><Trash2 size={14}/>Видалити</button></div>;
}

function ClipThumbnail({ clip, index }: { clip: CandidateClip; index: number }) {
  const [failed, setFailed] = useState(false);
  const url = api.fileUrl(clip.thumbnailPath);
  if (!url || failed) return <div className="generated-visual"><span>#{index + 1}</span></div>;
  return <img src={url} alt={`Preview моменту ${index + 1}`} onError={() => setFailed(true)}/>;
}

function ClipPreview({ clip, sourcePath, onClose, onEdit }: { clip: CandidateClip; sourcePath: string; onClose: () => void; onEdit: () => void }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const sourceUrl = api.fileUrl(sourcePath);
  function seekAndPlay() {
    const video = videoRef.current;
    if (!video) return;
    video.currentTime = clip.startSeconds;
    void video.play().catch(() => undefined);
  }
  function stopAtClipEnd() {
    const video = videoRef.current;
    if (video && video.currentTime >= clip.endSeconds) {
      video.pause();
      video.currentTime = clip.startSeconds;
    }
  }
  return <div className="clip-preview-backdrop" onMouseDown={event => event.target === event.currentTarget && onClose()}>
    <section className="clip-preview-dialog" role="dialog" aria-modal="true" aria-label="Перегляд моменту">
      <header><div><span className="eyebrow">Попередній перегляд</span><strong>{formatTime(clip.startSeconds)} — {formatTime(clip.endSeconds)}</strong></div><button className="icon-button" onClick={onClose}><X size={18}/></button></header>
      <div className="clip-preview-player">{sourceUrl ? <video ref={videoRef} src={sourceUrl} controls autoPlay preload="metadata" onLoadedMetadata={seekAndPlay} onTimeUpdate={stopAtClipEnd}/> : <div className="preview-unavailable"><Film size={36}/><span>Preview доступний у Windows-застосунку</span></div>}</div>
      <footer><span>Відтворюється фрагмент вихідного відео без попереднього рендера.</span><button className="primary" onClick={onEdit}><Pencil size={15}/>Редагувати</button></footer>
    </section>
  </div>;
}

function Field({ icon: Icon, label, children }: { icon: typeof Film; label: string; children: React.ReactNode }) {
  return <label className="analysis-field"><span><Icon size={14}/>{label}</span><div>{children}</div></label>;
}
