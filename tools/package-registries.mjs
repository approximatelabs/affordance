#!/usr/bin/env node
// Build the npm packages and the Python wheels that carry the prebuilt `affordance` command, from
// a release's archives (`affordance-v<version>-<target>.tar.gz`, as release.yml makes them).
//
//   node tools/package-registries.mjs <archives directory> <output directory>
//
// npm: one package, `affordance-rs`, holding every platform's binary (`bin/<os>-<cpu>/`), whose
// `affordance` command runs the one for the machine it is on. One package to register, at the
// cost of a larger download.
// PyPI: one wheel a platform (`affordance-<version>-py3-none-<platform>.whl`) with the binary as
// its script, so `pip install affordance` puts `affordance` on the PATH. No Python code, no build
// step: a wheel is a zip with a fixed layout, written here.
// Needs Node 22 and `tar`. No dependencies.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { crc32, deflateRawSync } from "node:zlib";

const NPM_NAME = "affordance-rs";
const PYPI_NAME = "affordance";
const HOME = "https://affordance.rs";
const REPOSITORY = "https://github.com/approximatelabs/affordance";
const SUMMARY = "A browser for models: apps in TSX, rendered to the exact token ids a model reads";

// A release target as each registry names it. The Linux wheels say the oldest glibc they run on:
// the one of the manylinux_2_28 image release.yml builds in. The macOS ones say Rust's
// deployment targets.
const TARGETS = {
  "x86_64-unknown-linux-gnu": { os: "linux", cpu: "x64", wheel: "manylinux_2_28_x86_64" },
  "aarch64-unknown-linux-gnu": { os: "linux", cpu: "arm64", wheel: "manylinux_2_28_aarch64" },
  "aarch64-apple-darwin": { os: "darwin", cpu: "arm64", wheel: "macosx_11_0_arm64" },
  "x86_64-apple-darwin": { os: "darwin", cpu: "x64", wheel: "macosx_10_12_x86_64" },
};

const [from, to] = process.argv.slice(2);
if (!from || !to) {
  console.error("usage: package-registries.mjs <archives directory> <output directory>");
  process.exit(2);
}

/** A zip archive of `files` ({name, data, mode}), each deflated. */
function zip(files) {
  const parts = [];
  const central = [];
  let offset = 0;
  for (const file of files) {
    const name = Buffer.from(file.name);
    const packed = deflateRawSync(file.data, { level: 9 });
    const head = Buffer.alloc(30);
    head.writeUInt32LE(0x04034b50, 0);
    head.writeUInt16LE(20, 4); // version needed
    head.writeUInt16LE(0x0800, 6); // names are UTF-8
    head.writeUInt16LE(8, 8); // deflate
    head.writeUInt16LE(0, 10); // time: fixed, so the same input makes the same file
    head.writeUInt16LE(0x21, 12); // date: 1980-01-01
    head.writeUInt32LE(crc32(file.data), 14);
    head.writeUInt32LE(packed.length, 18);
    head.writeUInt32LE(file.data.length, 22);
    head.writeUInt16LE(name.length, 26);
    const entry = Buffer.alloc(46);
    entry.writeUInt32LE(0x02014b50, 0);
    entry.writeUInt16LE((3 << 8) | 20, 4); // made on Unix: the mode below is read
    head.copy(entry, 6, 4, 30); // version needed .. name length
    entry.writeUInt32LE(((0o100000 | file.mode) << 16) >>> 0, 38);
    entry.writeUInt32LE(offset, 42);
    parts.push(head, name, packed);
    central.push(entry, name);
    offset += 30 + name.length + packed.length;
  }
  const directory = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...parts, directory, end]);
}

const digest = (data) => createHash("sha256").update(data).digest("base64url");

function wheel(version, tag, binary, license, readme, notices) {
  const info = `${PYPI_NAME}-${version}.dist-info`;
  const files = [
    { name: `${PYPI_NAME}-${version}.data/scripts/affordance`, data: binary, mode: 0o755 },
    {
      name: `${info}/METADATA`,
      mode: 0o644,
      data: Buffer.from(
        [
          "Metadata-Version: 2.4",
          `Name: ${PYPI_NAME}`,
          `Version: ${version}`,
          `Summary: ${SUMMARY}`,
          `Home-page: ${HOME}`,
          `Project-URL: Repository, ${REPOSITORY}`,
          "License-Expression: Apache-2.0",
          "License-File: LICENSE",
          "License-File: THIRD_PARTY_NOTICES.md",
          "Requires-Python: >=3.8",
          "Description-Content-Type: text/markdown",
          "",
          readme,
        ].join("\n"),
      ),
    },
    {
      name: `${info}/WHEEL`,
      mode: 0o644,
      data: Buffer.from(`Wheel-Version: 1.0\nGenerator: affordance package-registries\nRoot-Is-Purelib: false\nTag: py3-none-${tag}\n`),
    },
    { name: `${info}/licenses/LICENSE`, mode: 0o644, data: license },
    { name: `${info}/licenses/THIRD_PARTY_NOTICES.md`, mode: 0o644, data: notices },
  ];
  const record = files.map((f) => `${f.name},sha256=${digest(f.data)},${f.data.length}`).join("\n");
  files.push({ name: `${info}/RECORD`, mode: 0o644, data: Buffer.from(`${record}\n${info}/RECORD,,\n`) });
  return zip(files);
}

const LAUNCHER = `#!/usr/bin/env node
// Runs the affordance binary for this machine: this package holds one for each platform.
const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");
const binary = join(__dirname, process.platform + "-" + process.arch, "affordance");
if (!existsSync(binary)) {
  console.error("affordance: no prebuilt binary for " + process.platform + " " + process.arch + ".");
  console.error("Build it from source instead: cargo install affordance");
  process.exit(1);
}
const run = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (run.error) {
  console.error("affordance: " + run.error.message);
  process.exit(1);
}
if (run.signal) process.kill(process.pid, run.signal);
process.exit(run.status === null ? 1 : run.status);
`;

const archives = readdirSync(from).filter((name) => /^affordance-v.+\.tar\.gz$/.test(name));
if (!archives.length) {
  console.error(`no affordance-v*.tar.gz in ${from}`);
  process.exit(1);
}
rmSync(to, { recursive: true, force: true });
mkdirSync(join(to, "npm"), { recursive: true });
mkdirSync(join(to, "pypi"), { recursive: true });
const scratch = mkdtempSync(join(tmpdir(), "affordance-package-"));
const platforms = [];
const main = join(to, "npm", NPM_NAME);
mkdirSync(join(main, "bin"), { recursive: true });
let notices;
let version;
let license;
let readme;
for (const archive of archives.sort()) {
  const [, found, target] = archive.match(/^affordance-v(.+?)-([a-z0-9_]+-[a-z0-9_-]+)\.tar\.gz$/) ?? [];
  const names = TARGETS[target];
  if (!names) {
    console.error(`${archive}: not a target this script packages`);
    process.exit(1);
  }
  if (version && version !== found) {
    console.error(`${archive}: version ${found}, but another archive is ${version}`);
    process.exit(1);
  }
  version = found;
  execFileSync("tar", ["-xzf", join(from, archive), "-C", scratch]);
  const dir = join(scratch, archive.replace(/\.tar\.gz$/, ""));
  const binary = readFileSync(join(dir, "affordance"));
  license = readFileSync(join(dir, "LICENSE"));
  readme = readFileSync(join(dir, "README.md"), "utf8");

  const platform = `${names.os}-${names.cpu}`;
  mkdirSync(join(main, "bin", platform), { recursive: true });
  writeFileSync(join(main, "bin", platform, "affordance"), binary);
  chmodSync(join(main, "bin", platform, "affordance"), 0o755);
  notices = readFileSync(join(dir, "THIRD_PARTY_NOTICES.md"));
  platforms.push(platform);

  writeFileSync(join(to, "pypi", `${PYPI_NAME}-${version}-py3-none-${names.wheel}.whl`), wheel(version, names.wheel, binary, license, readme, notices));
  console.log(`${target}: ${platform}, ${names.wheel}`);
}

writeFileSync(join(main, "bin", "affordance.js"), LAUNCHER);
chmodSync(join(main, "bin", "affordance.js"), 0o755);
writeFileSync(join(main, "LICENSE"), license);
writeFileSync(join(main, "THIRD_PARTY_NOTICES.md"), notices);
writeFileSync(join(main, "README.md"), readme);
writeFileSync(
  join(main, "package.json"),
  `${JSON.stringify(
    {
      name: NPM_NAME,
      version,
      description: SUMMARY,
      license: "Apache-2.0",
      homepage: HOME,
      repository: { type: "git", url: `git+${REPOSITORY}.git` },
      bin: { affordance: "bin/affordance.js" },
      files: ["bin", "LICENSE", "THIRD_PARTY_NOTICES.md"],
      engines: { node: ">=18" },
    },
    null,
    2,
  )}\n`,
);
rmSync(scratch, { recursive: true, force: true });
console.log(`${NPM_NAME} ${version}: ${platforms.length} platforms in one package; wheels in ${join(to, "pypi")}`);
