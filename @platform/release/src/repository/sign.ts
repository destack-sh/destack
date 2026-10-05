import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFile } from "node:fs/promises";
import { readReleaseKeys } from "../key/key.ts";
import { readRootHistory } from "./renewal.ts";
import { Release } from "@destack/update/release";
import { listInstallers, readCommit, version } from "../distribution/index.ts";
import { RepositoryConfiguration } from "./index.ts";
import { writeBootstrap } from "./bootstrap.ts";
import { type Distribution, SignedRepository } from "@destack/update/publish";
import { print } from "../output/index.ts";

/** Repository directory containing the distribution build outputs. */
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

// load the online signing keys without the offline root key
/** Online keys authorized for the selected repository. */
const keys = await readReleaseKeys();
/** Publication destination selected by this invocation. */
const configuration = new RepositoryConfiguration();
/** Authenticated root history for the selected feed, ending at the root authorizing this release. */
const { history, root } = await readRootHistory(configuration.url, await configuration.root());

// prevent the stable signer from authorizing a nightly release or the reverse
if (
    new Release(version, Release.target(process.platform, process.arch)).channel !==
    configuration.channel
) {
    throw new Error("release version does not match the selected repository");
}

// sign the explicitly selected complete archives
if (process.argv.slice(2).length === 0) {
    throw new Error("select the targets to publish");
}
/** Directory containing this release's compiled artifacts. */
const directory = join(ROOT, "dist", version);
/** The commit every distribution was built from. */
const commit = await readCommit(ROOT);
/** Complete target matrix selected for publication. */
const distributions: Distribution[] = process.argv.slice(2).map((target) => {
    const release = new Release(version, target);

    return {
        target,
        version: release.version,
        commit,
        archive: join(directory, `destack-${release.directory}.tar.gz`),
    };
});
// sign each installer whose applications are all selected
for (const { installer, platforms } of listInstallers()) {
    if (platforms.every(({ target }) => process.argv.slice(2).includes(target))) {
        distributions.push({
            target: installer.name,
            version,
            commit,
            archive: join(directory, `destack-${version}-${installer.name}.${installer.format}`),
            format: installer.format,
        });
    }
}
/** Signed targets of the written repository, which its download catalog lists. */
const targets = await SignedRepository.create(
    join(ROOT, "dist/update"),
    Date.now(),
    root,
    keys,
    distributions,
);
await writeBootstrap(join(ROOT, "dist/update"), targets, configuration.url);
// retain every rotation needed by clients bootstrapping from the embedded root
for (const rotation of history) {
    await writeFile(
        join(ROOT, "dist/update/metadata", `${rotation.signed.version}.root.json`),
        SignedRepository.encode(rotation),
    );
}
print(`Signed Destack ${version} for ${distributions.length} targets.`);
