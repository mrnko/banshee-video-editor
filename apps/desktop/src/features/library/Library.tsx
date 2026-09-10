import { useMemo, useState } from "react";
import { FileAudio, FileVideo2, Folder, FolderPlus, Image, Pencil, Plus, Search, Trash2, X } from "lucide-react";
import { api } from "../../lib/api";
import type { Asset } from "../../lib/types";
import { useAppStore } from "../../state/useAppStore";

export function Library() {
  const { data, addAsset, addAssetFolder, replaceAsset, removeAsset, notify } = useAppStore();
  const [query, setQuery] = useState("");
  const [folder, setFolder] = useState("all");
  const [preview, setPreview] = useState<Asset>();
  const [menu, setMenu] = useState<{ asset: Asset; x: number; y: number }>();
  const folders = useMemo(() => [
    { id: "all", name: "Усі ресурси" }, { id: "video", name: "Відеовставки" },
    { id: "audio", name: "Музика та звуки" }, { id: "image", name: "Зображення" },
    ...(data?.assetFolders ?? []),
  ], [data?.assetFolders]);
  const assets = useMemo(() => (data?.assets ?? []).filter(asset => {
    const inFolder = folder === "all" || folder === asset.kind || asset.folderId === folder;
    return inFolder && asset.name.toLowerCase().includes(query.toLowerCase());
  }), [data?.assets, folder, query]);
  async function importAsset() {
    try {
      const path = await api.chooseAsset(); if (!path) return;
      const customFolder = data?.assetFolders.some(item => item.id === folder) ? folder : undefined;
      addAsset(await api.importAsset(path, customFolder)); notify("Ресурс додано до медіатеки");
    } catch (error) { notify(String(error)); }
  }
  async function createFolder() {
    const name = window.prompt("Назва директорії");
    if (name?.trim() && !folders.some(item => item.name === name.trim())) {
      const created = await api.createAssetFolder(name.trim()); addAssetFolder(created); setFolder(created.id); notify("Директорію створено");
    }
  }
  async function renameAsset() { if (!menu) return; const name = window.prompt("Нова назва ресурсу", menu.asset.name); if (!name?.trim()) return; try { replaceAsset(await api.renameAsset(menu.asset.id, name)); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  async function deleteAsset() { if (!menu || !window.confirm(`Видалити «${menu.asset.name}» з медіатеки? Оригінальний файл залишиться на диску.`)) return; try { await api.deleteAsset(menu.asset.id); removeAsset(menu.asset.id); } catch (error) { notify(String(error)); } finally { setMenu(undefined); } }
  return <main className="page" onClick={() => setMenu(undefined)}><header className="page-header compact"><div><span className="eyebrow">Локальні матеріали</span><h1>Медіатека</h1><p>Відео, зображення, музика та звуки на вашому комп’ютері.</p></div><button className="primary" onClick={importAsset}><Plus size={18}/>Додати ресурс</button></header>
    <div className="library-layout"><aside className="folder-panel"><div className="folder-title"><span>Директорії</span><button onClick={createFolder}><FolderPlus size={16}/></button></div>{folders.map(item => <button className={folder === item.id ? "active" : ""} key={item.id} onClick={() => setFolder(item.id)}><Folder size={16}/>{item.name}<span>{item.id === "all" ? (data?.assets.length ?? 0) : ""}</span></button>)}</aside>
      <section className="library-content"><div className="search"><Search size={17}/><input value={query} onChange={e => setQuery(e.target.value)} placeholder="Пошук у медіатеці"/></div>
        {assets.length ? <div className="asset-grid">{assets.map(asset => <article key={asset.id} className="asset-card" onClick={() => asset.kind !== "audio" && setPreview(asset)} onContextMenu={event => { event.preventDefault(); setMenu({ asset, x: event.clientX, y: event.clientY }); }}>
          <AssetVisual asset={asset}/><strong>{asset.name}</strong><span>{asset.kind === "video" ? "Відео" : asset.kind === "audio" ? "Аудіо" : "Зображення"}</span>
          {asset.kind === "audio" && <audio className="asset-audio" controls src={api.fileUrl(asset.path)} onClick={event => event.stopPropagation()}/>} {asset.missing && <em>Файл відсутній</em>}
        </article>)}</div> : <div className="empty-state library-empty"><div><Folder size={28}/></div><h3>У цій директорії порожньо</h3><p>Додайте відео, зображення, музику або звуки.</p><button className="secondary" onClick={importAsset}><Plus size={16}/>Додати ресурс</button></div>}
      </section></div>{preview && <AssetPreview asset={preview} onClose={() => setPreview(undefined)}/>} {menu && <div className="context-menu" style={{ left: menu.x, top: menu.y }} onClick={event => event.stopPropagation()}><button onClick={renameAsset}><Pencil size={14}/>Перейменувати</button><button className="danger" onClick={deleteAsset}><Trash2 size={14}/>Видалити</button></div>}</main>;
}

export function AssetVisual({ asset }: { asset: Asset }) {
  const url = api.fileUrl(asset.kind === "video" ? asset.thumbnailPath : asset.path);
  const sourceUrl = api.fileUrl(asset.path);
  if (asset.kind === "image") return url ? <div className="asset-visual"><img src={url} alt=""/></div> : <div className="asset-icon"><Image/></div>;
  if (asset.kind === "video") return <div className="asset-visual">{url ? <img src={url} alt=""/> : <video src={sourceUrl} muted playsInline preload="metadata" onLoadedMetadata={event => { event.currentTarget.currentTime = .1; }}/>}<span className="asset-play">▶</span></div>;
  return <div className="asset-icon"><FileAudio/></div>;
}

function AssetPreview({ asset, onClose }: { asset: Asset; onClose: () => void }) {
  const url = api.fileUrl(asset.path);
  return <div className="asset-preview-backdrop" onMouseDown={event => event.target === event.currentTarget && onClose()}><section className="asset-preview-dialog"><header><strong>{asset.name}</strong><button className="icon-button" onClick={onClose}><X size={18}/></button></header><div className="asset-preview-content">{asset.kind === "image" ? <img src={url} alt={asset.name}/> : <video src={url} controls autoPlay playsInline/>}</div></section></div>;
}
