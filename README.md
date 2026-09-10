# Banshee Video Editor

Локальний Windows-редактор для пошуку найкращих моментів у довгих відео та експорту вертикальних кліпів для TikTok, Reels і Shorts.

## Можливості v0.1.x

- локальний аналіз сцен, руху, тиші та динаміки;
- необов'язкове ранжування відібраних кадрів через OpenAI Responses API;
- автономний fallback без API;
- просте редагування: trim, видалення пауз, crop, zoom, текст, аудіо та chroma key;
- 720p, 1080p і source-aware export;
- системне Windows Share, відкриття папки та копіювання шляху після експорту;
- SQLite-проєкти, локальна медіатека, autosave і статистика витрат;
- installer та portable-архів для Windows x64.

## Запуск

Потрібні Node.js 20+, Rust stable MSVC, Microsoft C++ Build Tools і WebView2.

```powershell
npm install
npm run dev
```

Для нативного застосунку:

```powershell
npm run tauri -- dev
```

FFmpeg шукається у `PATH`, WinGet-папках або за шляхом `FFMPEG_PATH`. API-ключ вводиться лише в налаштуваннях застосунку та зберігається у Windows Credential Manager.

Real-ESRGAN підключається через `REALESRGAN_PATH` або `tools/bin/realesrgan-ncnn-vulkan.exe`. Якщо Vulkan чи модель недоступні, експорт автоматично продовжується через якісний Lanczos fallback із попередженням.

## Збірка

```powershell
npm run check
npm run build
npm run tauri -- build
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1
```

Перед публікацією додайте дозволені для розповсюдження FFmpeg/ffprobe та Real-ESRGAN у `tools/bin`; див. `tools/README.md` і `THIRD_PARTY_NOTICES.md`.

## Дані

Проєкти, кеш, медіатека та налаштування зберігаються у `%LOCALAPPDATA%\\Banshee Video Editor`. Вихідні відео не змінюються.

## Ліцензія

MIT. Сторонні медіа-інструменти мають власні ліцензії.
