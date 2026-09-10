import fs from "node:fs";

const expected = fs.readFileSync(new URL("../VERSION", import.meta.url), "utf8").trim();
const files = [
  ["package.json", JSON.parse(fs.readFileSync(new URL("../package.json", import.meta.url))).version],
  ["apps/desktop/package.json", JSON.parse(fs.readFileSync(new URL("../apps/desktop/package.json", import.meta.url))).version],
  ["apps/desktop/src-tauri/tauri.conf.json", JSON.parse(fs.readFileSync(new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url))).version]
];

const mismatches = files.filter(([, version]) => version !== expected);
if (mismatches.length) {
  console.error(`VERSION=${expected}; mismatch: ${mismatches.map(([file, version]) => `${file}=${version}`).join(", ")}`);
  process.exit(1);
}
console.log(`Banshee version ${expected} is synchronized.`);
