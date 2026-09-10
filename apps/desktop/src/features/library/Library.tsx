import { useMemo, useState } from "react";
import { FileAudio, FileVideo2, Folder, FolderPlus, Image, Plus, Search } from "lucide-react";
import { api } from "../../lib/api";
import { useAppStore } from "../../state/useAppStore";

export function Library() {
  const { data, addAsset, addAssetFolder, notify } = useAppStore();
  const [query, setQuery] = useState("");
  const [folder, setFolder] = useState("all");
  const folders = useMemo(() => [
    { id: "all", name: "Усі ресурси" },
    { id: "video", name: "Відеовставки" },
    { id: "audio", name: "Музика та звуки" },
    ...(data?.assetFolders ?? []),
  ], [data?.assetFolders]);
  const assets = useMemo(() => (data?.assets ?? []).filter(asset => {
    const inFolder = folder === "all" || folder === asset.kind || asset.folderId === folder;
    return inFolder && asset.name.toLowerCase().includes(query.toLowerCase());
  }), [data?.assets, folder, query]);
  async function importAsset() { try { const path = await api.chooseAsset(); if (!path) return; const customFolder = data?.assetFolders.some(item => item.id === folder) ? folder : undefined; addAsset(await api.importAsset(path, customFolder)); notify("Ресурс додано до медіатеки"); } catch(error) { notify(String(error)); } }
  async function createFolder() { const name = window.prompt("Назва директорії"); if (name?.trim() && !folders.some(item => item.name === name.trim())) { const created = await api.createAssetFolder(name.trim()); addAssetFolder(created); setFolder(created.id); notify("Директорію створено"); } }
  return <main className="page"><header className="page-header compact"><div><span className="eyebrow">Локальні матеріали</span><h1>Медіатека</h1><p>Відеовставки, музика та звуки зберігаються на вашому комп’ютері.</p></div><button className="primary" onClick={importAsset}><Plus size={18}/>Додати ресурс</button></header>
    <div className="library-layout"><aside className="folder-panel"><div className="folder-title"><span>Директорії</span><button onClick={createFolder}><FolderPlus size={16}/></button></div>{folders.map(item=><button className={folder===item.id?"active":""} key={item.id} onClick={()=>setFolder(item.id)}><Folder size={16}/>{item.name}<span>{item.id==="all"?(data?.assets.length ?? 0):""}</span></button>)}</aside><section className="library-content"><div className="search"><Search size={17}/><input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Пошук у медіатеці"/></div>{assets.length?<div className="asset-grid">{assets.map(asset=><article key={asset.id}><div className="asset-icon">{asset.kind==="video"?<FileVideo2/>:asset.kind==="audio"?<FileAudio/>:<Image/>}</div><strong>{asset.name}</strong><span>{asset.kind === "video" ? "Відео" : asset.kind === "audio" ? "Аудіо" : "Зображення"}</span>{asset.missing&&<em>Файл відсутній</em>}</article>)}</div>:<div className="empty-state library-empty"><div><Folder size={28}/></div><h3>У цій директорії порожньо</h3><p>Додайте green-screen вставки, музику або звуки.</p><button className="secondary" onClick={importAsset}><Plus size={16}/>Додати ресурс</button></div>}</section></div>
  </main>;
}
