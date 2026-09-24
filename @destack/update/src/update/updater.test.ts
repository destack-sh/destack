import { afterAll, beforeAll, expect, test } from "@destack/test";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { create } from "tar";
import { Metadata, Timestamp } from "@tufjs/models";
import { Repository, createRootKey } from "./tests/repository.ts";
import { Updater } from "./updater.ts";
import { Release } from "../release/release.ts";
import { createRoot } from "../../../../platform/release/src/key/root.ts";
import { SigningKey } from "../../../../platform/release/src/key/key.ts";
import {
    createRepository,
    encode,
} from "../../../../platform/release/src/repository/repository.ts";

/** Native executable filename expected by the selected archive format. */
const COMMAND = process.platform === "win32" ? "destack.exe" : "destack";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** Compiled executable used by the signed archive scenarios. */
let fixture: string;

beforeAll(async () => {
    // generate offline signers once for the entire fixture
    rootKeys = Array.from({ length: 3 }, createRootKey);

    // compile the native fixture before measuring update operations
    fixture = await mkdtemp(join(tmpdir(), "destack-update-fixture-"));
    const result = await Bun.build({
        entrypoints: [fileURLToPath(new URL("./tests/fixture.ts", import.meta.url))],
        compile: { outfile: join(fixture, COMMAND), autoloadDotenv: false, autoloadBunfig: false },
    });
    if (!result.success) {
        throw new AggregateError(result.logs, "cannot compile update fixture");
    }

    // archive the executable once for independent repository scenarios
    const source = join(fixture, "source");
    await mkdir(join(source, "bin"), { recursive: true });
    await mkdir(join(source, "Destack"));
    await cp(join(fixture, COMMAND), join(source, "bin", COMMAND));
    await create({ file: join(fixture, "release.tar.gz"), gzip: { level: 1 }, cwd: source }, [
        "bin",
        "Destack",
    ]);
}, 10000);

afterAll(async () => {
    await rm(fixture, { recursive: true, force: true });
});

/** Hash complete executable contents without retaining the runtime binary in memory. */
async function digest(path: string): Promise<string> {
    // stream each executable through SHA-256 once
    const hash = createHash("sha256");
    for await (const bytes of createReadStream(path)) {
        hash.update(bytes);
    }

    return hash.digest("hex");
}

test("retain staging across sessions and reject concurrent or stale activation", async () => {
    // open an independent signed repository and installation
    await using repository = await Repository.open(fixture, rootKeys);
    const { options, target } = repository;

    // stage the complete archive under an exclusive updater session
    {
        await using updater = await Updater.open(options);
        await expect(Updater.open(options)).rejects.toThrow("Another Destack update is running.");

        // serialize operations within the same session
        const checking = updater.check();
        await expect(updater.check()).rejects.toThrow("An update operation is already running.");
        const update = await checking;
        expect(update?.release).toEqual(new Release("2026.9.1", target));

        // retain the complete executable without selecting it
        const download = await update!.download();
        const installed = await updater.stage(download);
        expect(await updater.current()).toBeUndefined();
        expect(await updater.staged()).toEqual(installed);
        expect(await digest(join(installed.directory, "bin", COMMAND))).toBe(
            await digest(join(fixture, COMMAND)),
        );
    }

    // a directly launched newer executable must not activate an older staged distribution
    {
        await using updater = await Updater.open({
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

    // activate only after reopening the persisted staged release
    {
        await using updater = await Updater.open(options);
        const staged = await updater.staged();
        const installed = await updater.activate(staged!);
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
        expect(await updater.check()).toBeUndefined();
    }
});

test("retry cancelled downloads and reject tampered archives before staging", async () => {
    // open an independent signed repository and installation
    await using repository = await Repository.open(fixture, rootKeys);
    const { options, directory, archive } = repository;

    // reject cancellation before the download starts
    await using updater = await Updater.open(options);
    const update = await updater.check();
    const cancelled = new AbortController();
    cancelled.abort(new Error("Cancelled download."));
    await expect(update!.download({ signal: cancelled.signal })).rejects.toThrow(
        "Cancelled download.",
    );

    // cancel after bytes arrive and require the next attempt to download successfully
    const interrupted = new AbortController();
    await expect(
        update!.download({
            signal: interrupted.signal,
            onProgress: () => interrupted.abort(new Error("Interrupted download.")),
        }),
    ).rejects.toThrow("Interrupted download.");
    expect(await updater.current()).toBeUndefined();
    expect(await updater.staged()).toBeUndefined();

    // retry successfully and compare the complete download progress
    const progress: { received: number; total: number }[] = [];
    const download = await update!.download({
        onProgress: (value) => progress.push(value),
    });
    const length = (await readFile(archive)).length;
    expect(progress.at(-1)).toEqual({ received: length, total: length });
    expect(await updater.current()).toBeUndefined();

    // reject changed archive bytes before extracting or running them
    const modified = join(directory, "modified.tar.gz");
    await writeFile(modified, "tampered");
    await expect(updater.stage({ ...download, archive: modified })).rejects.toThrow(
        "Installer archive failed verification.",
    );

    // retain a verified archive after the rejected attempt
    const staged = await updater.stage(download);
    expect(await updater.staged()).toEqual(staged);
    expect(await updater.current()).toBeUndefined();
});

test("retain rotated trust across sessions and reject expired or rolled-back metadata", async () => {
    // retain the original timestamp before the repository rotates
    await using repository = await Repository.open(fixture, rootKeys);
    const { options, target, archive } = repository;

    const firstTimestamp = await readFile(join(repository.path, "metadata/timestamp.json"));

    // persist initial trust before rotating the repository
    {
        await using updater = await Updater.open(options);
        expect((await updater.check())?.release).toEqual(new Release("2026.9.1", target));
    }

    // rotate both offline and online signing roles
    repository.keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    const replacement = Array.from({ length: 3 }, () => SigningKey.generate());
    repository.root = createRoot(
        2,
        replacement.map((key) => key.public),
        {
            targets: repository.keys.targets.public,
            snapshot: repository.keys.snapshot.public,
            timestamp: repository.keys.timestamp.public,
        },
        new Date(Date.now() + 365 * 86_400_000).toISOString(),
    );
    for (const key of [...rootKeys.slice(0, 2), ...replacement.slice(0, 2)]) {
        repository.root.sign((bytes) => key.sign(bytes));
    }
    await createRepository(repository.path, 2, repository.root, repository.keys, [
        {
            target,
            version: "2026.9.1",
            archive,
        },
    ]);

    // retain rotated trust across independent updater sessions
    {
        await using updater = await Updater.open(options);
        expect((await updater.check())?.release).toEqual(new Release("2026.9.1", target));
    }

    // reject signed expired metadata without selecting or staging another release
    const timestampPath = join(repository.path, "metadata/timestamp.json");
    const timestamp = new Metadata(
        Timestamp.fromJSON({
            ...JSON.parse(await readFile(timestampPath, "utf8")).signed,
            version: 3,
            expires: "2000-01-01T00:00:00Z",
        }),
    );
    timestamp.sign((bytes) => repository.keys.timestamp.sign(bytes));
    await writeFile(timestampPath, encode(timestamp));
    {
        await using updater = await Updater.open(options);
        await expect(updater.check()).rejects.toThrow("Final timestamp.json is expired");
        expect(await updater.current()).toBeUndefined();
    }

    // a valid signature must not permit rollback of persisted metadata
    const rollback = new Metadata(Timestamp.fromJSON(JSON.parse(firstTimestamp.toString()).signed));
    rollback.sign((bytes) => repository.keys.timestamp.sign(bytes));
    await writeFile(timestampPath, encode(rollback));
    {
        await using updater = await Updater.open(options);
        await expect(updater.check()).rejects.toThrow(
            "New timestamp version 1 is less than current version 2",
        );
        expect(await updater.current()).toBeUndefined();
        expect(await updater.staged()).toBeUndefined();
    }
});

test("preserve the installed release when signed replacements contradict its identity", async () => {
    // open an independent signed repository and installation
    await using repository = await Repository.open(fixture, rootKeys);
    const { options, target, source, archive } = repository;
    let installed: Awaited<ReturnType<Updater["activate"]>>;

    // establish an active release before publishing inconsistent replacements
    {
        await using updater = await Updater.open(options);
        const update = await updater.check();
        installed = await updater.activate(await updater.stage(await update!.download()));
    }

    // a signed archive with the wrong executable version must leave the installation unchanged
    await createRepository(repository.path, 3, repository.root, repository.keys, [
        {
            target,
            version: "2026.9.2",
            archive,
        },
    ]);
    {
        await using updater = await Updater.open(options);
        const update = await updater.check();
        const download = await update!.download();
        await expect(updater.stage(download)).rejects.toThrow(
            "Downloaded CLI version does not match the signed release.",
        );
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
    }

    // reject changed bytes published under the installed version across sessions
    await writeFile(join(source, "Destack/changed"), "changed release contents");
    await create({ file: archive, gzip: { level: 1 }, cwd: source }, ["bin", "Destack"]);
    await createRepository(repository.path, 4, repository.root, repository.keys, [
        {
            target,
            version: "2026.9.1",
            archive,
        },
    ]);
    {
        await using updater = await Updater.open(options);
        await expect(updater.check()).rejects.toThrow("Published release changed.");
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
    }
});
