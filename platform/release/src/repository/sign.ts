import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFile } from "node:fs/promises";
import { readReleaseKeys } from "../key/key.ts";
import { createRepository, encode, type Distribution } from "./repository.ts";
import { readRootHistory } from "./renewal.ts";
import { Release } from "@destack/update/release";
import { version } from "../distribution/index.ts";
import { RepositoryConfiguration } from "./index.ts";

/** Repository directory containing the distribution build outputs. */
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

// load only the automated signing keys; the root private key stays offline
/** Online keys authorized for the selected repository. */
const keys = await readReleaseKeys();
/** Publication destination selected by this invocation. */
const configuration = new RepositoryConfiguration();
/** Authenticated root history for the selected feed. */
const history = await readRootHistory(configuration.url, await configuration.root());
/** Current root authorizing this release's online signing keys. */
const root = history[history.length - 1]!;

// prevent the stable signer from authorizing a nightly release or the reverse
if (new Release(version, Release.target()).channel !== configuration.channel) {
    throw new Error("release version does not match the selected repository");
}

// sign the explicitly selected complete archives
if (process.argv.slice(2).length === 0) {
    throw new Error("select the targets to publish");
}
/** Directory containing this release's compiled artifacts. */
const directory = join(ROOT, "dist", version);
/** Complete target matrix selected for publication. */
const distributions: Distribution[] = process.argv.slice(2).map((target) => {
    const release = new Release(version, target);

    return {
        target,
        version: release.version,
        archive: join(directory, `destack-${release.directory}.tar.gz`),
    };
});
if (
    process.argv.slice(2).includes("aarch64-apple-darwin") &&
    process.argv.slice(2).includes("x86_64-apple-darwin")
) {
    distributions.push({
        target: "universal-apple-darwin",
        version,
        archive: join(directory, `destack-${version}-universal-apple-darwin.dmg`),
        format: "dmg",
    });
}
// authenticate the graphical Windows installer independently of its updater archive
if (process.argv.slice(2).includes("x86_64-pc-windows-msvc")) {
    distributions.push({
        target: "x86_64-pc-windows-msvc",
        version,
        archive: join(directory, `destack-${version}-x86_64-pc-windows-msvc.exe`),
        format: "exe",
    });
}
await createRepository(
    join(ROOT, "dist/update"),
    Date.now(),
    root,
    keys,
    distributions,
    configuration.url,
);
// retain every rotation needed by clients bootstrapping from the embedded root
for (const root of history) {
    await writeFile(
        join(ROOT, "dist/update/metadata", `${root.signed.version}.root.json`),
        encode(root),
    );
}
console.log(`Signed Destack ${version} for ${distributions.length} targets.`);
