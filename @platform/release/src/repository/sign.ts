import { CHECKOUT } from "../distribution/distribution.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { join } from "node:path";
import { readReleaseKeys } from "../key/key.ts";
import { RootHistory } from "./renewal.ts";
import { Release } from "@destack/update/release";
import { Installer, readCommit, releaseVersion } from "../distribution/index.ts";
import { RepositoryConfiguration } from "./index.ts";
import { writeBootstrap } from "./bootstrap.ts";
import { type Distribution, SignedRepository } from "@destack/update/publish";
import { print } from "../distribution/output.ts";
import { Version } from "@destack/schema";

/** Sign the selected targets' archives and their installers into `dist/update`. */
export async function signUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // read the version every component of the distribution ships at
    const version = await releaseVersion(CHECKOUT, environment);
    const selected = arguments_;

    // load the online signing keys without the offline root key
    const keys = await readReleaseKeys(environment);
    const configuration = new RepositoryConfiguration(environment);

    // read the feed's authenticated root history, ending at the root authorizing this release
    const { history, root } = await RootHistory.read(
        configuration.url,
        configuration.root(),
        environment,
    );

    // refuse a dev build, which the feed never publishes
    if (Version.isPrerelease(version)) {
        throw new Error(`the feed publishes no dev build: ${version}`);
    }

    // sign the explicitly selected complete archives
    if (selected.length === 0) {
        throw new Error("select the targets to publish");
    }
    const directory = join(CHECKOUT, "dist", version);
    const commit = await readCommit(CHECKOUT, environment);
    const distributions: Distribution[] = selected.map((target) => {
        const release = new Release(version, target);

        return {
            target,
            version: release.version,
            commit,
            archive: join(directory, `destack-${release.directory}.tar.gz`),
        };
    });

    // sign each installer whose applications are all selected
    for (const { installer, platforms } of Installer.list()) {
        if (platforms.every(({ target }) => selected.includes(target))) {
            distributions.push({
                target: installer.name,
                version,
                commit,
                archive: join(
                    directory,
                    `destack-${version}-${installer.name}.${installer.format}`,
                ),
                format: installer.format,
            });
        }
    }

    // write the signed repository and the download catalog of its targets
    const targets = await SignedRepository.create(
        join(CHECKOUT, "dist/update"),
        environment.clock.now(),
        root,
        keys,
        distributions,
        environment,
    );
    await writeBootstrap(
        join(CHECKOUT, "dist/update"),
        targets,
        configuration.url,
        environment.files,
    );

    // retain every rotation needed by clients bootstrapping from the embedded root
    for (const rotation of history) {
        await environment.files.writeFile(
            join(CHECKOUT, "dist/update/metadata", `${rotation.signed.version}.root.json`),
            SignedRepository.encode(rotation),
        );
    }
    print(`Signed Destack ${version} for ${distributions.length} targets.`, environment);
}
