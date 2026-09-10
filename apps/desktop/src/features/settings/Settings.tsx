import { useEffect, useState } from "react";
import { Check, Cpu, Eye, EyeOff, KeyRound, RefreshCw, RotateCcw, Save, ShieldCheck, Sparkles } from "lucide-react";
import { api } from "../../lib/api";
import { DEFAULT_PROMPT, type AppSettings, type ModelOption } from "../../lib/types";
import { useAppStore } from "../../state/useAppStore";

export function Settings() {
  const { data, setData, notify } = useAppStore();
  const [settings, setSettings] = useState<AppSettings>(data?.settings ?? { autosaveSeconds: 15, model: "gpt-5.6-terra", adminCostSync: false, prompt: DEFAULT_PROMPT });
  const [apiKey, setApiKey] = useState(""); const [adminKey, setAdminKey] = useState("");
  const [showKey, setShowKey] = useState(false); const [models, setModels] = useState<ModelOption[]>([]); const [loadingModels, setLoadingModels] = useState(false);
  useEffect(()=>{ if(data) setSettings(data.settings); },[data]);
  if (!data) return null;
  const bootstrap = data;
  async function saveAll() {
    try { await api.saveSettings(settings); if(apiKey) await api.setApiKey("openai",apiKey); if(adminKey) await api.setApiKey("admin",adminKey); setData({...bootstrap,settings,apiKeyConfigured:bootstrap.apiKeyConfigured||!!apiKey}); setApiKey(""); setAdminKey(""); notify("Налаштування збережено"); }
    catch(error){notify(String(error));}
  }
  async function refreshModels(){setLoadingModels(true);try{setModels(await api.models());}catch(error){notify(String(error));}finally{setLoadingModels(false)}}
  async function syncCosts(){try{const total=await api.syncCosts();notify(`Офіційні витрати за 30 днів: $${total.toFixed(2)}`);}catch(error){notify(String(error));}}
  return <main className="page settings-page"><header className="page-header compact"><div><span className="eyebrow">Banshee v{data.version}</span><h1>Налаштування</h1><p>AI, autosave, локальні інструменти та приватність.</p></div><button className="primary" onClick={saveAll}><Save size={17}/>Зберегти</button></header>
    <div className="settings-grid"><section className="settings-card"><div className="settings-card-title"><div className="setting-icon"><KeyRound size={19}/></div><div><h2>OpenAI API</h2><p>Ключі зберігаються у Windows Credential Manager.</p></div>{data.apiKeyConfigured&&<span className="connected"><Check size={13}/>Підключено</span>}</div>
      <label className="setting-field"><span>API key</span><div className="password-input"><input type={showKey?"text":"password"} value={apiKey} onChange={e=>setApiKey(e.target.value)} placeholder={data.apiKeyConfigured?"Збережений ключ · введіть для заміни":"sk-…"}/><button onClick={()=>setShowKey(!showKey)}>{showKey?<EyeOff size={17}/>:<Eye size={17}/>}</button></div></label>
      <div className="inline-setting"><label className="setting-field"><span>Модель за замовчуванням</span><select value={settings.model} onChange={e=>setSettings({...settings,model:e.target.value})}><option>gpt-5.6-terra</option><option>gpt-6-astra</option><option>gpt-5.6-sol</option><option>gpt-5.6-luna</option></select></label><button className="secondary refresh-models" onClick={refreshModels}><RefreshCw size={16} className={loadingModels?"spin":""}/>Перевірити</button></div>
      {!!models.length&&<div className="model-results">{models.slice(0,8).map(m=><span key={m.id} className={m.available?"available":""}>{m.id}{m.recommended&&" · рекомендовано"}</span>)}</div>}
    </section>
    <section className="settings-card"><div className="settings-card-title"><div className="setting-icon"><ShieldCheck size={19}/></div><div><h2>Синхронізація витрат</h2><p>Необов’язкова звірка з OpenAI Costs API.</p></div></div><label className="switch-row"><div><strong>Admin cost sync</strong><span>Не частіше одного разу на 15 хвилин</span></div><input type="checkbox" checked={settings.adminCostSync} onChange={e=>setSettings({...settings,adminCostSync:e.target.checked})}/></label>{settings.adminCostSync&&<><label className="setting-field"><span>Admin API key</span><input type="password" value={adminKey} onChange={e=>setAdminKey(e.target.value)} placeholder="Введіть окремий admin key"/></label><button className="secondary" onClick={syncCosts}><RefreshCw size={15}/>Звірити зараз</button></>}</section>
    <section className="settings-card"><div className="settings-card-title"><div className="setting-icon"><Sparkles size={19}/></div><div><h2>Промпт highlights-v1</h2><p>Інструкція для змістового аналізу кандидатів.</p></div><button className="reset" onClick={()=>setSettings({...settings,prompt:DEFAULT_PROMPT})}><RotateCcw size={14}/>Скинути</button></div><textarea className="prompt-area" value={settings.prompt} onChange={e=>setSettings({...settings,prompt:e.target.value})}/></section>
    <section className="settings-card"><div className="settings-card-title"><div className="setting-icon"><Cpu size={19}/></div><div><h2>Локальна обробка</h2><p>Стан зовнішніх медіаінструментів.</p></div></div><div className="tool-status"><div><span className={data.ffmpegAvailable?"dot ok":"dot"}/><div><strong>FFmpeg / ffprobe</strong><span>{data.ffmpegAvailable?"Готовий до роботи":"Не знайдено"}</span></div></div><div><span className={data.upscalerAvailable?"dot ok":"dot warning"}/><div><strong>Real-ESRGAN Vulkan</strong><span>{data.upscalerAvailable?"AI-upscale доступний":"Буде використано Lanczos fallback"}</span></div></div></div><label className="setting-field short"><span>Autosave, секунд</span><input type="number" min="5" max="300" value={settings.autosaveSeconds} onChange={e=>setSettings({...settings,autosaveSeconds:+e.target.value})}/></label></section></div>
  </main>;
}
