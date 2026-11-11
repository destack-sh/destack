import { CHECKOUT } from "../distribution/distribution.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { join } from "node:path";
import { readReleaseKeys } from "../key/key.ts";
import { RootHistory } from "./renewal.ts";
import { Release, type Target } from "@destack/update/release";
import { Installer, Platform, readCommit, releaseVersion } from "../distribution/index.ts";
import { Distribution, Manifest, TOOLCHAIN_DIRECTORY } from "@destack/update";
import { run } from "../distribution/command.ts";
import { RepositoryConfiguration } from "./index.ts";
import { writeBootstrap } from "./bootstrap.ts";
import { SignedRepository } from "@destack/update/publish";
import type * as publish from "@destack/update/publish";
import { print } from "../distribution/output.ts";
import { Version } from "@destack/schema";

/** Sign the selected targets' releases, archives and installers into `dist/update`. */
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
        throw new Error(`the feed publishes no dev build: '${version}'`);
    }

    // sign each selected target's release from its signed archive, the archive for install.sh, and its toolchain
    if (selected.length === 0) {
        throw new Error("select the targets to publish");
    }
    const directory = join(CHECKOUT, "dist", version);
    const commit = await readCommit(CHECKOUT, environment);
    const unpacked = await environment.files.makeTemporaryDirectory("destack-release-");
    const distributions: publish.Distribution[] = [];
    for (const target of selected) {
        const release = new Release(version, target);
        const archive = join(directory, `destack-${release.directory}.tar.gz`);
        const files = join(unpacked, target);
        await environment.files.mkdir(files);
        await run("tar", ["-xzf", archive, "-C", files], environment);
        const toolchain = await readToolchain(directory, files, release.target, environment);
        distributions.push(
            { kind: "release", target, version, commit, directory: files },
            { kind: "installer", target, version, commit, archive, format: "tar.gz" },
            { kind: "toolchain", target, version, commit, ...toolchain },
        );
    }

    // sign each installer whose applications are all selected
    for (const { installer, platforms } of Installer.list()) {
        if (platforms.every(({ target }) => selected.includes(target))) {
            distributions.push({
                kind: "installer",
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

    // write the signed repository, then remove the unpacked releases
    let targets: Awaited<ReturnType<typeof SignedRepository.create>>;
    try {
        targets = await SignedRepository.create(
            join(CHECKOUT, "dist/update"),
            environment.clock.now(),
            root,
            keys,
            distributions,
            environment,
        );
    } finally {
        await environment.files.remove(unpacked, { recursive: true, force: true });
    }

    // write the download catalog of its targets
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

/** Read a target's toolchain: its files from its CI artifact, its manifest from its release. */
async function readToolchain(
    directory: string,
    release: string,
    target: Target,
    environment: Environment<"files">,
): Promise<{ readonly directory: string; readonly manifest: Manifest }> {
    // read the manifest the release's resources list for its architecture
    const { bundle, architecture, signer } = Platform.of(target);
    const helpers = join(release, bundle.application, bundle.helpers);
    const resources = Distribution.resources(join(helpers, "destack"));
    const listed = join(resources, Distribution.toolchainManifest(architecture));
    const manifest = Manifest.parse(await environment.files.readTextFile(listed));

    // take the files from the artifact of the job that signed them, else the build's
    const artifact = `${TOOLCHAIN_DIRECTORY}-${signer === undefined ? "unsigned" : "signed"}-${target}`;

    return { directory: join(directory, TOOLCHAIN_DIRECTORY, artifact), manifest };
}
