import { expect, test } from "@destack/test";
import { createServer } from "node:http";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { create } from "tar";
import { Metadata, Timestamp } from "@tufjs/models";
import { Updater } from "./updater.ts";
import { Release } from "../release/release.ts";
import { SigningKey } from "../../../../dev/release/src/sign/key.ts";
import {
    createRepository,
    createRoot,
    encode,
} from "../../../../dev/release/src/sign/repository.ts";

test("stage across sessions, rotate trust, and preserve the active release on rejection", async () => {
    // serve signed distributions from a temporary directory
    const directory = await mkdtemp(join(tmpdir(), "destack-updater-"));
    const repository = join(directory, "repository");
    const server = createServer(async (request, response) => {
        try {
            response.end(await readFile(join(repository, request.url!)));
        } catch (error) {
            response.statusCode = (error as NodeJS.ErrnoException).code === "ENOENT" ? 404 : 500;
            response.end();
        }
    });
    try {
        await new Promise<void>((resolve, reject) => {
            server.once("error", reject);
            server.listen(0, "127.0.0.1", resolve);
        });
        const address = server.address();
        if (!address || typeof address === "string") throw new Error("Missing server address.");

        // build an executable fixture and a complete signed archive
        const source = join(directory, "source");
        await mkdir(join(source, "bin"), { recursive: true });
        await mkdir(join(source, "Destack"));
        const executable = join(source, "bin/destack");
        await writeFile(executable, '#!/bin/sh\nprintf \'{"version":"2026.9.1"}\\n\'\n');
        await chmod(executable, 0o755);
        const archive = join(directory, "release.tar.gz");
        await create({ file: archive, gzip: true, cwd: source }, ["bin", "Destack"]);

        let keys = {
            targets: SigningKey.generate(),
            snapshot: SigningKey.generate(),
            timestamp: SigningKey.generate(),
        };
        const rootKey = SigningKey.generate();
        let root = createRoot(1, rootKey, keys);
        const target = "x86_64-unknown-linux-gnu";
        await createRepository(repository, 1, root, keys, [{
            target,
            version: "2026.9.1",
            archive,
        }]);
        const firstTimestamp = await readFile(join(repository, "metadata/timestamp.json"));
        const options = {
            directory: join(directory, "installation"),
            repository: new URL(`http://127.0.0.1:${address.port}/`),
            root: encode(root).toString(),
            target,
        } as const;

        // exercise the public API, progress, cancellation, and installation locking
        {
            using updater = await Updater.open(options);
            await expect(Updater.open(options)).rejects.toThrow(
                "Another Destack update is running.",
            );
            const checking = updater.check();
            await expect(updater.check()).rejects.toThrow(
                "An update operation is already running.",
            );
            const update = await checking;
            expect(update?.release).toEqual(new Release("2026.9.1", target));
            const cancelled = new AbortController();
            cancelled.abort(new Error("Cancelled download."));
            await expect(update!.download({ signal: cancelled.signal })).rejects.toThrow(
                "Cancelled download.",
            );

            // cancel after bytes arrive and require the next attempt to download successfully
            const interrupted = new AbortController();
            await expect(update!.download({
                signal: interrupted.signal,
                onProgress: () => interrupted.abort(new Error("Interrupted download.")),
            })).rejects.toThrow("Interrupted download.");
            expect(await updater.current()).toBeUndefined();
            expect(await updater.staged()).toBeUndefined();

            const progress: { received: number; total: number }[] = [];
            const download = await update!.download({
                onProgress: (value) => progress.push(value),
            });
            const length = (await readFile(archive)).length;
            expect(progress.at(-1)).toEqual({ received: length, total: length });
            expect(await updater.current()).toBeUndefined();
            const modified = join(directory, "modified.tar.gz");
            await writeFile(modified, "tampered");
            await expect(updater.stage({ ...download, archive: modified })).rejects.toThrow(
                "Installer archive failed verification.",
            );
            const installed = await updater.stage(download);
            expect(await updater.current()).toBeUndefined();
            expect(await updater.staged()).toEqual(installed);
            expect(await readFile(join(installed.directory, "bin/destack"), "utf8"))
                .toBe('#!/bin/sh\nprintf \'{"version":"2026.9.1"}\\n\'\n');
        }

        // a directly launched newer executable must not activate an older staged distribution
        {
            using updater = await Updater.open({
                ...options,
                current: new Release("2026.9.2", target),
            });
            const staged = await updater.staged();
            await expect(updater.activate(staged!)).rejects.toThrow(
                "Refusing to activate a release older than the running version.",
            );
            expect(await updater.current()).toBeUndefined();
            expect(await updater.staged()).toEqual(staged);
        }

        // rotate trust while the verified release waits for activation
        keys = {
            targets: SigningKey.generate(),
            snapshot: SigningKey.generate(),
            timestamp: SigningKey.generate(),
        };
        root = createRoot(2, SigningKey.generate(), keys, rootKey);
        await createRepository(repository, 2, root, keys, [{
            target,
            version: "2026.9.1",
            archive,
        }]);

        // reopening preserves staging and follows the signed root rotation
        {
            using updater = await Updater.open(options);
            expect(await updater.current()).toBeUndefined();
            const staged = await updater.staged();
            const installed = await updater.activate(staged!);
            expect(await updater.current()).toEqual(installed);
            expect(await updater.staged()).toBeUndefined();
            expect(await updater.check()).toBeUndefined();
            expect((await updater.current())?.release).toEqual(new Release("2026.9.1", target));
        }

        // a signed archive with the wrong executable version must leave the installation unchanged
        await createRepository(repository, 3, root, keys, [{
            target,
            version: "2026.9.2",
            archive,
        }]);
        {
            using updater = await Updater.open(options);
            const update = await updater.check();
            const download = await update!.download();
            await expect(updater.stage(download)).rejects.toThrow(
                "Downloaded CLI version does not match the signed release.",
            );
            expect((await updater.current())?.release).toEqual(new Release("2026.9.1", target));
        }

        // reject changed bytes published under the installed version across sessions
        await writeFile(join(source, "Destack/changed"), "changed release contents");
        await create({ file: archive, gzip: true, cwd: source }, ["bin", "Destack"]);
        await createRepository(repository, 4, root, keys, [{
            target,
            version: "2026.9.1",
            archive,
        }]);
        {
            using updater = await Updater.open(options);
            await expect(updater.check()).rejects.toThrow("Published release changed.");
            expect((await updater.current())?.release).toEqual(new Release("2026.9.1", target));
        }

        // reject signed expired metadata without selecting or staging another release
        const timestampPath = join(repository, "metadata/timestamp.json");
        const timestamp = new Metadata(Timestamp.fromJSON({
            ...JSON.parse(await readFile(timestampPath, "utf8")).signed,
            version: 5,
            expires: "2000-01-01T00:00:00Z",
        }));
        timestamp.sign((bytes) => keys.timestamp.sign(bytes));
        await writeFile(timestampPath, encode(timestamp));
        {
            using updater = await Updater.open(options);
            await expect(updater.check()).rejects.toThrow("Final timestamp.json is expired");
            expect((await updater.current())?.release).toEqual(new Release("2026.9.1", target));
        }

        // a valid signature must not permit rollback of persisted metadata
        const rollback = new Metadata(
            Timestamp.fromJSON(JSON.parse(firstTimestamp.toString()).signed),
        );
        rollback.sign((bytes) => keys.timestamp.sign(bytes));
        await writeFile(timestampPath, encode(rollback));
        {
            using updater = await Updater.open(options);
            await expect(updater.check()).rejects.toThrow(
                "New timestamp version 1 is less than current version 4",
            );
            expect((await updater.current())?.release).toEqual(new Release("2026.9.1", target));
            expect(await updater.staged()).toBeUndefined();
        }
    } finally {
        server.closeAllConnections();
        if (server.listening) {
            await new Promise<void>((resolve, reject) =>
                server.close((error) => error ? reject(error) : resolve())
            );
        }
        await rm(directory, { recursive: true, force: true });
    }
});
