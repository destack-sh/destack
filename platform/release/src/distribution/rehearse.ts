import { ReleaseIdentity } from "./identity.ts";
import { Updater, Release } from "@destack/update";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { SigningKey } from "../key/key.ts";
import { createRoot } from "../key/root.ts";
import { createRepository, renewMetadata } from "../repository/repository.ts";

/** Exercise a real archive through authenticated download, separate staging and activation sessions. */
async function rehearse(): Promise<void> {
    // generate disposable signing roles entirely in memory
    const [version, archive] = process.argv.slice(2);
    if (!version || !archive) {
        throw new Error("usage: rehearse.ts <version> <archive>");
    }
    const release = new Release(version, Release.target());
    const roots = Array.from({ length: 3 }, () => SigningKey.generate());
    const keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    const root = createRoot(
        1,
        roots.map((key) => key.public),
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(Date.now() + 86400000).toISOString(),
    );
    root.sign((bytes) => roots[0]!.sign(bytes));
    root.sign((bytes) => roots[1]!.sign(bytes));

    // serve only the temporary authenticated repository on loopback
    const directory = await mkdtemp(join(tmpdir(), "destack-install-rehearsal-"));
    const repository = join(directory, "repository");
    const server = Bun.serve({
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
    try {
        await createRepository(repository, 1, root, keys, [
            {
                target: release.target,
                version: release.version,
                archive: resolve(archive),
            },
        ]);
        const options = {
            directory: join(directory, "distribution"),
            repository: new URL(server.url),
            root: JSON.stringify(root.toJSON()),
            target: release.target,
            applicationIdentifier: new ReleaseIdentity(release.channel).applicationIdentifier,
            application: process.platform === "darwin" ? join(directory, "Destack.app") : undefined,
        };

        // stage the real executables without activating them in the first process lifetime
        {
            await using updater = await Updater.open(options);
            const update = await updater.check();
            if (!update) {
                throw new Error("rehearsal release was not offered");
            }
            await updater.stage(await update.download());
            if (await updater.current()) {
                throw new Error("staging unexpectedly activated the application");
            }
        }

        // renew freshness without the targets key and activate from persisted staging
        const targets = await readFile(join(repository, "metadata/1.targets.json"));
        await renewMetadata(
            repository,
            2,
            root,
            { snapshot: keys.snapshot, timestamp: keys.timestamp },
            targets,
        );
        {
            await using updater = await Updater.open(options);
            const staged = await updater.staged();
            if (!staged || staged.release.version !== release.version) {
                throw new Error("staged release did not survive reopening");
            }
            await updater.activate(staged);
            if (
                (await updater.current())?.release.version !== release.version ||
                (await updater.check())
            ) {
                throw new Error("activation did not select the authenticated release");
            }
        }
        console.log(
            `verified real-archive staging, renewal and activation for ${release.directory}`,
        );
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
}

await rehearse();
