import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { UpdateInfo } from "./types";
import { useAppStore } from "../state/useAppStore";

const REPOSITORY = "mrnko/banshee-video-editor";
const DISMISSED_KEY = "banshee.dismissedUpdateVersion";
const LAST_CHECK_KEY = "banshee.lastUpdateCheck";
export const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

let pendingUpdate: Update | null = null;

export function updateMetadata(version: string, body?: string, date?: string): UpdateInfo {
  const lines = (body ?? "").trim().split(/\r?\n/);
  const title = (lines.shift() || `Banshee ${version}`).replace(/^#+\s*/, "").trim();
  const notes = lines.join("\n").trim() || "Виправлення та покращення редактора.";
  const fileName = `Banshee-Video-Editor-${version}-portable.zip`;
  return {
    version,
    title,
    notes,
    date,
    portableUrl: `https://github.com/${REPOSITORY}/releases/download/v${version}/${encodeURIComponent(fileName)}`,
  };
}

export function dismissUpdate() {
  const { updater, setUpdater } = useAppStore.getState();
  if (updater.info) localStorage.setItem(DISMISSED_KEY, updater.info.version);
  setUpdater({ modalOpen: false });
}

export function showAvailableUpdate() {
  if (useAppStore.getState().updater.info) useAppStore.getState().setUpdater({ modalOpen: true });
}

export async function checkForUpdates(manual = false) {
  const { setUpdater, notify } = useAppStore.getState();
  if (!isTauri()) {
    if (manual) notify("Перевірка оновлень доступна у Windows-застосунку");
    return null;
  }
  setUpdater({ status: "checking", error: undefined });
  try {
    pendingUpdate?.close().catch(() => undefined);
    pendingUpdate = await check({ timeout: 15_000 });
    const checkedAt = new Date().toISOString();
    localStorage.setItem(LAST_CHECK_KEY, checkedAt);
    if (!pendingUpdate) {
      setUpdater({ status: "idle", info: undefined, modalOpen: false, lastCheckedAt: checkedAt });
      if (manual) notify("У вас встановлена остання версія Banshee");
      return null;
    }
    const info = updateMetadata(pendingUpdate.version, pendingUpdate.body, pendingUpdate.date);
    const dismissed = localStorage.getItem(DISMISSED_KEY) === info.version;
    setUpdater({ status: "available", info, modalOpen: manual || !dismissed, lastCheckedAt: checkedAt, downloadedBytes: 0, totalBytes: undefined });
    return info;
  } catch (error) {
    const message = `Не вдалося перевірити оновлення: ${String(error)}`;
    setUpdater({ status: "error", error: message, modalOpen: false });
    if (manual) notify(message);
    return null;
  }
}

export async function installAvailableUpdate() {
  const { data, busy, updater, setUpdater, notify } = useAppStore.getState();
  if (!updater.info) return;
  if (data?.portable) {
    await openUrl(updater.info.portableUrl);
    return;
  }
  if (busy) {
    notify("Дочекайтеся завершення поточної обробки або рендеру");
    return;
  }
  if (!pendingUpdate) {
    await checkForUpdates(true);
    if (!pendingUpdate) return;
  }
  try {
    const tasks: Promise<unknown>[] = [];
    window.dispatchEvent(new CustomEvent("banshee-before-update", { detail: { tasks } }));
    await Promise.all(tasks);
    let downloadedBytes = 0;
    setUpdater({ status: "downloading", downloadedBytes: 0, error: undefined, modalOpen: true });
    await pendingUpdate.download(event => {
      if (event.event === "Started") setUpdater({ totalBytes: event.data.contentLength });
      if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength;
        setUpdater({ downloadedBytes });
      }
      if (event.event === "Finished") setUpdater({ status: "installing", downloadedBytes });
    });
    setUpdater({ status: "installing" });
    await pendingUpdate.install({ restartAfterInstall: true });
    await relaunch();
  } catch (error) {
    const message = `Не вдалося встановити оновлення: ${String(error)}`;
    setUpdater({ status: "error", error: message, modalOpen: true });
    notify(message);
  }
}

export function lastUpdateCheck() {
  return localStorage.getItem(LAST_CHECK_KEY) ?? undefined;
}
