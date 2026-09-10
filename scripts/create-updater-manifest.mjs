import fs from "node:fs";
import path from "node:path";

const [version, repository = process.env.GITHUB_REPOSITORY] = process.argv.slice(2);
if (!version || !repository) throw new Error("Version and repository are required");
const nsisDir = "target/release/bundle/nsis";
const installer = fs.readdirSync(nsisDir).find(name => name.includes(version) && name.endsWith("-setup.exe"));
if (!installer) throw new Error("NSIS installer not found");
const signaturePath = path.join(nsisDir, `${installer}.sig`);
if (!fs.existsSync(signaturePath)) throw new Error("Updater signature not found");
const notes = fs.readFileSync("RELEASE_NOTES.md", "utf8").trim();
const tag = `v${version}`;
const url = `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(installer)}`;
const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: fs.readFileSync(signaturePath, "utf8").trim(),
      url,
    },
  },
};
fs.mkdirSync("dist", { recursive: true });
fs.writeFileSync("dist/latest.json", `${JSON.stringify(manifest, null, 2)}\n`);
console.log("dist/latest.json created");
