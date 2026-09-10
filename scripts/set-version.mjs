import fs from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version ?? "")) {
  throw new Error("Usage: node scripts/set-version.mjs <semver>");
}

function updateJson(path) {
  const value = JSON.parse(fs.readFileSync(path, "utf8"));
  value.version = version;
  fs.writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

updateJson("package.json");
updateJson("apps/desktop/package.json");
updateJson("apps/desktop/src-tauri/tauri.conf.json");
fs.writeFileSync("VERSION", `${version}\n`);
const cargo = fs.readFileSync("Cargo.toml", "utf8").replace(/(\[workspace\.package\][\s\S]*?\nversion = ")[^"]+("\r?\n)/, `$1${version}$2`);
fs.writeFileSync("Cargo.toml", cargo);
console.log(`Banshee build version set to ${version}`);
