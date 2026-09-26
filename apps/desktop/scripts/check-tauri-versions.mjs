// The Tauri CLI refuses to build when the `tauri` crate and the
// `@tauri-apps/api` package differ in major.minor version. Dependabot
// updates Cargo and npm in separate pull requests, so one side can move
// without the other, and plain `cargo build` in CI would not notice: only
// the release would fail. This check makes such a pull request fail early.
import { readFileSync } from "node:fs";

const cargoLock = readFileSync(new URL("../../../Cargo.lock", import.meta.url), "utf8");
const npmLock = JSON.parse(readFileSync(new URL("../package-lock.json", import.meta.url), "utf8"));

const crate = /\[\[package\]\]\nname = "tauri"\nversion = "([^"]+)"/.exec(cargoLock)?.[1];
const npm = npmLock.packages?.["node_modules/@tauri-apps/api"]?.version;
const majorMinor = (version) => version.split(".").slice(0, 2).join(".");

if (!crate || !npm) {
  console.error("Could not find the tauri crate in Cargo.lock or @tauri-apps/api in package-lock.json.");
  process.exit(1);
}
if (majorMinor(crate) !== majorMinor(npm)) {
  console.error(`The tauri crate is ${crate} but @tauri-apps/api is ${npm}: move both to the same major.minor.`);
  process.exit(1);
}
console.log(`tauri ${crate} and @tauri-apps/api ${npm} match.`);
