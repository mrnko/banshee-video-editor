import fs from "node:fs";

const expected = fs.readFileSync(new URL("../VERSION", import.meta.url), "utf8").trim();
const files = [
  ["package.json", JSON.parse(fs.readFileSync(new URL("../package.json", import.meta.url))).version],
  ["apps/desktop/package.json", JSON.parse(fs.readFileSync(new URL("../apps/desktop/package.json", import.meta.url))).version],
  ["apps/desktop/src-tauri/tauri.conf.json", JSON.parse(fs.readFileSync(new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url))).version],
  ["Cargo.toml", fs.readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8").match(/\[workspace\.package\][\s\S]*?\nversion = "([^"]+)"/)?.[1]]
];

const mismatches = files.filter(([, version]) => version !== expected);
if (mismatches.length) {
  console.error(`VERSION=${expected}; mismatch: ${mismatches.map(([file, version]) => `${file}=${version}`).join(", ")}`);
  process.exit(1);
}
const notes = fs.readFileSync(new URL("../RELEASE_NOTES.md", import.meta.url), "utf8").trim();
if (!notes.startsWith("# ") || notes.split(/\r?\n/).length < 3) {
  console.error("RELEASE_NOTES.md must contain a title and at least one change");
  process.exit(1);
}
console.log(`Banshee version ${expected} is synchronized.`);
