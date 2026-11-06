import {
    type Environment,
    FileSystemError,
    type HostCapability,
    type Listener,
    Pipe,
} from "@destack/environment";
import { selectedChannel } from "./identity.ts";
import { Updater, type UpdaterOptions, Release } from "@destack/update";
import { join, resolve, sep } from "node:path";
import {
    type ReleaseKeys,
    SignedRepository,
    SigningKey,
    TrustedRoot,
} from "@destack/update/publish";
import type { Metadata, Root } from "@tufjs/models";
import { print } from "./output.ts";
import { readCommit } from "./distribution.ts";

/** The milliseconds a rehearsal root stays valid: one day. */
const ROOT_LIFETIME_MILLISECONDS = 86_400_000;

/** Download, stage and activate a real archive in separate authenticated sessions. */
export async function rehearseUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // generate disposable signing roles entirely in memory
    const { crypto, files, host } = environment;
    const [version, archive] = arguments_;
    if (version === undefined || archive === undefined) {
        throw new Error("usage: distribution rehearse <version> <archive>");
    }
    const release = new Release(version, Release.target(host.os, host.architecture));
    const keys = {
        targets: await SigningKey.generate("ed25519", crypto),
        snapshot: await SigningKey.generate("ed25519", crypto),
        timestamp: await SigningKey.generate("ed25519", crypto),
    };
    const root = await createSignedRoot(keys, environment);

    // serve only the temporary authenticated repository on loopback
    const directory = await files.makeTemporaryDirectory("destack-install-rehearsal-");
    const repository = join(directory, "repository");
    const server = serveRepository(repository, environment);
    try {
        // publish the archive as the only release of a fresh repository
        await SignedRepository.create(
            repository,
            1,
            root,
            keys,
            [
                {
                    target: release.target,
                    version: release.version,
                    commit: await readCommit(import.meta.dirname, environment),
                    archive: resolve(archive),
                },
            ],
            environment,
        );
        const options = {
            directory: join(directory, "distribution"),
            repository: new URL(server.url),
            root: JSON.stringify(root.toJSON()),
            target: release.target,
            applicationIdentifier: selectedChannel(environment).applicationIdentifier,
            ...(host.os === "darwin" && { application: join(directory, "Destack.app") }),
            environment,
        };

        // stage the release without activating it in the first process lifetime
        await stageUpdate(options);

        // renew freshness without the targets key
        const targets = Buffer.from(
            await files.readFile(join(repository, "metadata/1.targets.json")),
        );
        await SignedRepository.renew(
            repository,
            2,
            root,
            { snapshot: keys.snapshot, timestamp: keys.timestamp },
            targets,
            environment,
        );

        // activate from persisted staging in a new process lifetime
        await activateStaged(options, release.version);
        print(
            `verified real-archive staging, renewal and activation for ${release.directory}`,
            environment,
        );
    } finally {
        await server.stop(true);
        await files.remove(directory, { recursive: true, force: true });
    }
}

/** Create a one-day root over the online keys, signed by two of three disposable root keys. */
async function createSignedRoot(
    keys: ReleaseKeys,
    environment: Environment<HostCapability>,
): Promise<Metadata<Root>> {
    // generate the disposable root quorum
    const { crypto } = environment;
    const [first, second, third] = [
        await SigningKey.generate("ed25519", crypto),
        await SigningKey.generate("ed25519", crypto),
        await SigningKey.generate("ed25519", crypto),
    ];
    const root = TrustedRoot.create(
        1,
        [first.public, second.public, third.public],
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(environment.clock.now() + ROOT_LIFETIME_MILLISECONDS).toISOString(),
        environment,
    );

    // meet the two-of-three threshold
    await first.sign(root);
    await second.sign(root);

    return root;
}

/** Serve the files of a repository directory on a loopback port. */
function serveRepository(
    repository: string,
    environment: Environment<"files" | "server">,
): Listener {
    return environment.server.http<undefined>({
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

            // open the file, answering a missing one as not found
            const file = await environment.files
                .open(path, { read: true })
                .catch((error: unknown) => FileSystemError.absent(error));
            if (file === undefined) {
                return new Response(null, { status: 404 });
            }

            // stream the file, closing it once sent
            const { size } = await file.stat();
            const closing = new TransformStream<Uint8Array, Uint8Array>({
                flush: () => file.close(),
            });
            const body = Pipe.through(file.readable(), closing);

            return new Response(body, { headers: { "content-length": String(size) } });
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
