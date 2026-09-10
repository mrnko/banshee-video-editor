import { CheckCircle2, Download, LoaderCircle, RefreshCw, X } from "lucide-react";
import { dismissUpdate, installAvailableUpdate, showAvailableUpdate } from "../lib/updater";
import { useAppStore } from "../state/useAppStore";

function UpdateNotes({ notes }: { notes: string }) {
  const lines = notes.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
  return <div className="update-notes">{lines.map((line, index) => <p key={`${index}-${line}`}>{line.replace(/^[-*]\s+/, "")}</p>)}</div>;
}

export function UpdateCenter() {
  const { updater, data } = useAppStore();
  const { info, status, modalOpen, downloadedBytes, totalBytes, error } = updater;
  if (!info) return null;
  const percent = totalBytes ? Math.min(100, Math.round(downloadedBytes / totalBytes * 100)) : undefined;
  const working = status === "downloading" || status === "installing";
  return <>
    {!modalOpen && <button className="update-available-chip" onClick={showAvailableUpdate}><Download size={16}/><span>Доступна версія {info.version}</span></button>}
    {modalOpen && <div className="update-backdrop"><section className="update-dialog" role="dialog" aria-modal="true" aria-labelledby="update-title">
      <header><div className="update-icon">{working ? <LoaderCircle className="spin"/> : <RefreshCw/>}</div><div><span className="eyebrow">Нове оновлення</span><h2 id="update-title">{info.title}</h2><p>Версія {info.version}{info.date ? ` · ${new Date(info.date).toLocaleDateString("uk-UA")}` : ""}</p></div>{!working && <button className="icon-button" onClick={dismissUpdate}><X size={18}/></button>}</header>
      <UpdateNotes notes={info.notes}/>
      {working && <div className="update-progress"><div><span>{status === "downloading" ? "Завантаження…" : "Перевірка та встановлення…"}</span><strong>{status === "installing" ? "Готово" : percent === undefined ? "…" : `${percent}%`}</strong></div><div className="update-progress-track"><i style={{ width: status === "installing" ? "100%" : `${percent ?? 8}%` }}/></div></div>}
      {error && <p className="update-error">{error}</p>}
      <footer>{!working && <><button className="secondary" onClick={dismissUpdate}>Не зараз</button><button className="primary" onClick={() => void installAvailableUpdate()}>{data?.portable ? <Download size={16}/> : <RefreshCw size={16}/>} {data?.portable ? "Завантажити portable" : "Оновити та перезапустити"}</button></>}</footer>
      {data?.portable && <p className="update-portable-note"><CheckCircle2 size={14}/>Portable-версія не замінюється автоматично.</p>}
    </section></div>}
  </>;
}
