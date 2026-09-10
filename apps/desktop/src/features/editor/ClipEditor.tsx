import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Check, ChevronLeft, Clipboard, Film, FolderOpen, Layers3, Play, Plus, Save, Scissors, Share2, SlidersHorizontal, Sparkles, Type, Volume2, X } from "lucide-react";
import { api } from "../../lib/api";
import { formatTime } from "../../lib/format";
import type { CandidateClip, EditDecisionList, Overlay, Project } from "../../lib/types";
import { useAppStore } from "../../state/useAppStore";

export function ClipEditor({ clip, project, presetId, onClose }: { clip: CandidateClip; project: Project; presetId: string; onClose: () => void }) {
  const { data, notify } = useAppStore();
  const [edit, setEdit] = useState<EditDecisionList>(() => freshEdit(clip));
  const [tab, setTab] = useState<"crop"|"text"|"assets">("crop");
  const [savedAt, setSavedAt] = useState<Date>();
  const [rendering, setRendering] = useState(false);
  const [output, setOutput] = useState<string>();
  const [cutStart, setCutStart] = useState(clip.startSeconds + 4);
  const [cutEnd, setCutEnd] = useState(clip.startSeconds + 5);
  const revisionRef = useRef(1);
  const foregroundVideoRef = useRef<HTMLVideoElement>(null);
  const backgroundVideoRef = useRef<HTMLVideoElement>(null);
  useEffect(() => { api.loadEdit(clip.id).then(value => value && setEdit(value)); }, [clip.id]);
  const saveEdit = useCallback(async () => {
    const next = { ...edit, revision: ++revisionRef.current };
    await api.saveEdit(next); setSavedAt(new Date());
  }, [edit]);
  useEffect(() => { const timer = window.setInterval(() => saveEdit().catch(()=>{}), (data?.settings.autosaveSeconds ?? 15) * 1000); return () => window.clearInterval(timer); }, [data?.settings.autosaveSeconds, saveEdit]);
  const textOverlay = edit.overlays.find(o => o.type === "text") as Extract<Overlay,{type:"text"}> | undefined;
  const videoOverlay = edit.overlays.find(o => o.type === "video") as Extract<Overlay,{type:"video"}> | undefined;
  const fileUrl = api.fileUrl(project.source.path);
  const clipDuration = edit.trimEnd - edit.trimStart;
  const update = <K extends keyof EditDecisionList>(key: K, value: EditDecisionList[K]) => setEdit(current => ({ ...current, [key]: value }));
  const updateCrop = (patch: Partial<EditDecisionList["crop"]>) => update("crop", { ...edit.crop, ...patch });
  const syncBlurredBackground = (play = false) => {
    const foreground = foregroundVideoRef.current;
    const background = backgroundVideoRef.current;
    if (!foreground || !background) return;
    if (Math.abs(background.currentTime - foreground.currentTime) > .08) background.currentTime = foreground.currentTime;
    if (play) void background.play().catch(()=>{}); else background.pause();
  };
  function removeInterval() {
    const start = Math.max(edit.trimStart, Math.min(cutStart, edit.trimEnd));
    const end = Math.max(start, Math.min(cutEnd, edit.trimEnd));
    if (end - start < .1 || end - start >= clipDuration) { notify("Вкажіть коректний внутрішній інтервал"); return; }
    update("removedRanges", [...edit.removedRanges, { start, end }].sort((a,b)=>a.start-b.start));
    notify("Фрагмент буде видалено під час рендера");
  }
  function upsertText(patch: Partial<Extract<Overlay,{type:"text"}>>) {
    const current = textOverlay ?? { type: "text", id: crypto.randomUUID(), text: "Ваш текст", color: "#ffffff", size: 56, x: .5, y: .16, start: 0, end: clipDuration };
    update("overlays", [...edit.overlays.filter(o => o.type !== "text"), { ...current, ...patch }]);
  }
  async function addAsset() {
    const selected = data?.assets[0]; if (!selected) { notify("Спочатку додайте ресурс у медіатеку"); return; }
    const overlay: Overlay = selected.kind === "audio"
      ? { type: "audio", id: crypto.randomUUID(), assetId: selected.id, start: 0, end: clipDuration, volume: .7 }
      : { type: "video", id: crypto.randomUUID(), assetId: selected.id, start: 0, end: Math.min(clipDuration, 8), x: .5, y: .5, scale: .45, chromaKey: { color: "#00ff00", similarity: .18, blend: .08, spill: .15 } };
    update("overlays", [...edit.overlays.filter(o => o.type !== "video" || overlay.type !== "video"), overlay]);
  }
  async function render() {
    setRendering(true);
    try { await saveEdit(); const job = await api.render(project.id, clip.id, presetId); setOutput(job.outputPath); notify(job.warning ?? "Відео успішно відрендерено"); }
    catch (error) { if (String(error).includes("RENDER_CANCELLED")) return; notify(String(error)); }
    finally { setRendering(false); }
  }
  return <div className="editor-backdrop"><section className="editor-shell">
    <header className="editor-header"><div><button className="icon-button" onClick={onClose}><ChevronLeft size={20}/></button><div><span className="eyebrow">Редактор кліпу</span><strong>{project.name}</strong></div></div><div className="save-state">{savedAt ? <><Check size={14}/>Збережено {savedAt.toLocaleTimeString("uk-UA",{hour:"2-digit",minute:"2-digit"})}</> : "Autosave увімкнено"}</div><div><button className="secondary" onClick={() => saveEdit()}><Save size={16}/>Зберегти</button><button className="primary" onClick={render} disabled={rendering}><Sparkles size={17}/>{rendering ? "Рендер…" : "Експортувати"}</button><button className="icon-button" onClick={onClose}><X size={19}/></button></div></header>
    <div className="editor-main">
      <aside className="editor-tools"><button className={tab==="crop"?"active":""} onClick={()=>setTab("crop")}><SlidersHorizontal size={18}/>Кадр</button><button className={tab==="text"?"active":""} onClick={()=>setTab("text")}><Type size={18}/>Текст</button><button className={tab==="assets"?"active":""} onClick={()=>setTab("assets")}><Layers3 size={18}/>Ресурси</button></aside>
      <div className="preview-stage"><div className={`phone-preview mode-${edit.crop.mode}`} style={{ "--zoom": edit.crop.zoom } as React.CSSProperties}>{fileUrl ? <>{edit.crop.mode === "blurredBackground" && <video ref={backgroundVideoRef} className="blur-background-video" src={`${fileUrl}#t=${edit.trimStart}`} muted playsInline aria-hidden="true" />}<video ref={foregroundVideoRef} className="editor-preview-video" src={`${fileUrl}#t=${edit.trimStart}`} controls playsInline onPlay={()=>syncBlurredBackground(true)} onPause={()=>syncBlurredBackground(false)} onSeeking={()=>syncBlurredBackground(!foregroundVideoRef.current?.paused)} onTimeUpdate={()=>syncBlurredBackground(!foregroundVideoRef.current?.paused)} /></> : <div className="video-placeholder"><Film size={42}/></div>}{textOverlay && <div className="preview-text" style={{ color: textOverlay.color, fontSize: Math.max(14,textOverlay.size/3), left:`${textOverlay.x*100}%`, top:`${textOverlay.y*100}%` }}>{textOverlay.text}</div>}<div className="safe-zone"/></div></div>
      <aside className="inspector">
        {tab === "crop" && <><div className="inspector-title"><div><SlidersHorizontal size={18}/>Кадрування</div><span>9:16</span></div><Control label="Режим"><select value={edit.crop.mode} onChange={e=>updateCrop({mode:e.target.value as EditDecisionList["crop"]["mode"]})}><option value="center">По центру</option><option value="dynamic">Динамічний</option><option value="blurredBackground">Розмитий фон</option></select></Control><Control label={`Zoom · ${edit.crop.zoom.toFixed(2)}×`}><input type="range" min="1" max="3" step=".05" value={edit.crop.zoom} onChange={e=>updateCrop({zoom:+e.target.value})}/></Control><p className="hint">AI-upscale активується під час експорту, якщо ефективної роздільності недостатньо.</p></>}
        {tab === "text" && <><div className="inspector-title"><div><Type size={18}/>Текстовий шар</div></div><Control label="Текст"><textarea value={textOverlay?.text ?? ""} placeholder="Додайте текст…" onChange={e=>upsertText({text:e.target.value})}/></Control><div className="row-controls"><Control label="Колір"><input type="color" value={textOverlay?.color ?? "#ffffff"} onChange={e=>upsertText({color:e.target.value})}/></Control><Control label="Розмір"><input type="number" min="18" max="160" value={textOverlay?.size ?? 56} onChange={e=>upsertText({size:+e.target.value})}/></Control></div><Control label="Позиція по вертикалі"><input type="range" min="0" max=".9" step=".01" value={textOverlay?.y ?? .16} onChange={e=>upsertText({y:+e.target.value})}/></Control>{textOverlay && <button className="danger-link" onClick={()=>update("overlays",edit.overlays.filter(o=>o.id!==textOverlay.id))}>Видалити текст</button>}</>}
        {tab === "assets" && <><div className="inspector-title"><div><Layers3 size={18}/>Ресурси</div><span>{edit.overlays.filter(o=>o.type!=="text").length}</span></div><button className="resource-add" onClick={addAsset}><Plus size={18}/><div><strong>Додати з медіатеки</strong><span>Відео, музика або звук</span></div></button>{edit.overlays.filter(o=>o.type!=="text").map(o=><div className="layer-row" key={o.id}>{o.type==="audio"?<Volume2 size={17}/>:<Film size={17}/>}<span>{o.type==="audio"?"Аудіошар":"Відеовставка"}</span><button onClick={()=>update("overlays",edit.overlays.filter(x=>x.id!==o.id))}><X size={15}/></button></div>)}{videoOverlay?.chromaKey && <><Control label="Колір chroma key"><input type="color" value={videoOverlay.chromaKey.color} onChange={e=>update("overlays",edit.overlays.map(o=>o.id===videoOverlay.id?{...videoOverlay,chromaKey:{...videoOverlay.chromaKey!,color:e.target.value}}:o))}/></Control><Control label={`Tolerance · ${videoOverlay.chromaKey.similarity.toFixed(2)}`}><input type="range" min=".02" max=".6" step=".01" value={videoOverlay.chromaKey.similarity} onChange={e=>update("overlays",edit.overlays.map(o=>o.id===videoOverlay.id?{...videoOverlay,chromaKey:{...videoOverlay.chromaKey!,similarity:+e.target.value}}:o))}/></Control><Control label={`Softness · ${videoOverlay.chromaKey.blend.toFixed(2)}`}><input type="range" min="0" max=".5" step=".01" value={videoOverlay.chromaKey.blend} onChange={e=>update("overlays",edit.overlays.map(o=>o.id===videoOverlay.id?{...videoOverlay,chromaKey:{...videoOverlay.chromaKey!,blend:+e.target.value}}:o))}/></Control><Control label={`Green spill · ${videoOverlay.chromaKey.spill.toFixed(2)}`}><input type="range" min="0" max="1" step=".01" value={videoOverlay.chromaKey.spill} onChange={e=>update("overlays",edit.overlays.map(o=>o.id===videoOverlay.id?{...videoOverlay,chromaKey:{...videoOverlay.chromaKey!,spill:+e.target.value}}:o))}/></Control></>}</>}
      </aside>
    </div>
    <footer className="timeline"><div className="timeline-toolbar"><button><Play fill="currentColor" size={14}/></button><span>{formatTime(edit.trimStart)} / {formatTime(edit.trimEnd)}</span><div className="timeline-spacer"/><label className="inline-time">Від <input type="number" step=".1" value={cutStart} onChange={e=>setCutStart(+e.target.value)}/></label><label className="inline-time">до <input type="number" step=".1" value={cutEnd} onChange={e=>setCutEnd(+e.target.value)}/></label><button onClick={removeInterval}><Scissors size={14}/>Видалити фрагмент</button></div><div className="timeline-track"><div className="track-label"><Film size={15}/>VIDEO</div><div className="track-clip" style={{width:"78%"}}><span>{project.source.fileName}</span><div className="wave-lines"/></div></div>{edit.removedRanges.length > 0 && <div className="removed-ranges">{edit.removedRanges.map((range,index)=><button key={`${range.start}-${range.end}`} onClick={()=>update("removedRanges",edit.removedRanges.filter((_,i)=>i!==index))}><Scissors size={12}/>{formatTime(range.start)}–{formatTime(range.end)}<X size={12}/></button>)}</div>}<div className="trim-controls"><label>In <input type="number" step=".1" value={edit.trimStart} onChange={e=>update("trimStart",+e.target.value)}/></label><span>{formatTime(clipDuration)}</span><label>Out <input type="number" step=".1" value={edit.trimEnd} onChange={e=>update("trimEnd",+e.target.value)}/></label></div></footer>
    {output && <div className="render-complete"><Check size={18}/><span>Файл готовий</span><button onClick={()=>api.share(output).catch(error=>notify(String(error)))}><Share2 size={16}/>Поділитися</button><button onClick={()=>navigator.clipboard.writeText(output).then(()=>notify("Шлях скопійовано"))}><Clipboard size={16}/>Копіювати шлях</button><button onClick={()=>api.reveal(output)}><FolderOpen size={16}/>Показати</button></div>}
  </section></div>;
}

function freshEdit(clip: CandidateClip): EditDecisionList { return { projectId: clip.projectId, candidateId: clip.id, trimStart: clip.startSeconds, trimEnd: clip.endSeconds, removedRanges: [], crop: { mode: "center", zoom: 1, centerX: .5, centerY: .5 }, overlays: [], revision: 1 }; }
function Control({label,children}:{label:string;children:React.ReactNode}) { return <label className="control"><span>{label}</span>{children}</label>; }
