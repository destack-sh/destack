import { selectedChannel } from "./identity.ts";
import { Updater, type UpdaterOptions, Release } from "@destack/update";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import {
    type ReleaseKeys,
    SignedRepository,
    SigningKey,
    TrustedRoot,
} from "@destack/update/publish";
import type { Metadata, Root } from "@tufjs/models";
import { print } from "../output/index.ts";
import { readCommit } from "./distribution.ts";

/** Exercise a real archive through authenticated download, separate staging and activation sessions. */
async function rehearse(): Promise<void> {
    // generate disposable signing roles entirely in memory
    const [version, archive] = process.argv.slice(2);
    if (version === undefined || archive === undefined) {
        throw new Error("usage: rehearse.ts <version> <archive>");
    }
    const release = new Release(version, Release.target(process.platform, process.arch));
    const keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    const root = createSignedRoot(keys);

    // serve only the temporary authenticated repository on loopback
    const directory = await mkdtemp(join(tmpdir(), "destack-install-rehearsal-"));
    const repository = join(directory, "repository");
    const server = serveRepository(repository);
    try {
        // publish the archive as the only release of a fresh repository
        await SignedRepository.create(repository, 1, root, keys, [
            {
                target: release.target,
                version: release.version,
                commit: await readCommit(import.meta.dirname),
                archive: resolve(archive),
            },
        ]);
        const options = {
            directory: join(directory, "distribution"),
            repository: new URL(server.url),
            channel: release.channel,
            root: JSON.stringify(root.toJSON()),
            target: release.target,
            applicationIdentifier: selectedChannel().applicationIdentifier,
            ...(process.platform === "darwin" && { application: join(directory, "Destack.app") }),
        };

        // stage the release without activating it in the first process lifetime
        await stageUpdate(options);

        // renew freshness without the targets key
        const targets = await readFile(join(repository, "metadata/1.targets.json"));
        await SignedRepository.renew(
            repository,
            2,
            root,
            { snapshot: keys.snapshot, timestamp: keys.timestamp },
            targets,
        );

        // activate from persisted staging in a new process lifetime
        await activateStaged(options, release.version);
        print(`verified real-archive staging, renewal and activation for ${release.directory}`);
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
}

/** Create a one-day root over the online keys and sign it with two of three disposable root keys. */
function createSignedRoot(keys: ReleaseKeys): Metadata<Root> {
    // generate the disposable root quorum
    const [first, second, third] = [
        SigningKey.generate(),
        SigningKey.generate(),
        SigningKey.generate(),
    ];
    const root = TrustedRoot.create(
        1,
        [first.public, second.public, third.public],
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(Date.now() + 86400000).toISOString(),
    );

    // meet the two-of-three threshold
    root.sign((bytes) => first.sign(bytes));
    root.sign((bytes) => second.sign(bytes));

    return root;
}

/** Serve the files of a repository directory on a loopback port. */
function serveRepository(repository: string): Bun.Server<undefined> {
    return Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            // constrain the loopback server to generated repository files
            const path = resolve(
                repository,
                `.${decodeURIComponent(new URL(request.url).pathname)}`,
            );
            if (!path.startsWith(repository + sep)) {
                return new Response(null, { status: 400 });
            }
            const file = Bun.file(path);

            return (await file.exists()) ? new Response(file) : new Response(null, { status: 404 });
        },
    });
}

/** Stage the offered release without activating it in this process lifetime. */
async function stageUpdate(options: UpdaterOptions): Promise<void> {
    // stage the real executables from the offered update
    await using updater = await Updater.open(options);
    const update = await updater.check();
    if (!update) {
        throw new Error("rehearsal release was not offered");
    }
    await updater.stage(await update.download());

    // require the current application to stay unchanged
    if (await updater.current()) {
        throw new Error("staging unexpectedly activated the application");
    }
}

/** Activate the release persisted in staging and require it as the current release. */
async function activateStaged(options: UpdaterOptions, version: string): Promise<void> {
    // reopen the staged release
    await using updater = await Updater.open(options);
    const staged = await updater.staged();
    if (!staged || staged.release.version !== version) {
        throw new Error("staged release did not survive reopening");
    }

    // activate it and require no further update
    await updater.activate(staged);
    if ((await updater.current())?.release.version !== version || (await updater.check())) {
        throw new Error("activation did not select the authenticated release");
    }
}

await rehearse();
