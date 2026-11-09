import { testHost } from "@destack/test/host";
import { afterAll, beforeAll, expect, test, refusal } from "@destack/test";
import { join } from "node:path";
import { Digest } from "@destack/schema";
import { Pipe } from "@destack/environment";
import { Metadata, MetadataKind, Timestamp } from "@tufjs/models";
import { COMMIT, TestUpdate, createRootKey } from "../test/index.ts";
import { Updater } from "./updater.ts";
import type { Update } from "./update.ts";
import type { StagedRelease } from "../install/installer.ts";
import { parseObject } from "../publish/json.ts";
import { Release } from "../release/release.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "../publish/index.ts";

/** Native executable filename the fixture release contains. */
const COMMAND = testHost().host.os === "win32" ? "destack.exe" : "destack";

/** Offline signers shared as immutable key material. */
let rootKeys: SigningKey[];
/** Compiled executable used by the signed release scenarios. */
let fixture: string;

beforeAll(async () => {
    // generate offline signers once for the entire fixture
    rootKeys = await Promise.all(Array.from({ length: 3 }, () => createRootKey(testHost().crypto)));

    // compile the native fixture before measuring update operations
    fixture = await TestUpdate.compile();
}, 10000);

afterAll(async () => {
    await testHost().files.remove(fixture, { recursive: true });
});

/** Hash complete executable contents without retaining the runtime binary in memory. */
async function digest(path: string): Promise<string> {
    // stream each executable through SHA-256 once
    const { crypto, files } = testHost();
    await using file = await files.open(path, { read: true });
    const hash = await crypto.digest("SHA-256", file.readable());

    return Buffer.from(hash).toString("hex");
}

/** Check for an update, failing when none is available. */
async function checkUpdate(updater: Updater): Promise<Update> {
    const update = await updater.check();
    if (update === undefined) {
        throw new Error("expected an available update");
    }

    return update;
}

/** Read the staged release, failing when none is staged. */
async function readStaged(updater: Updater): Promise<StagedRelease> {
    const staged = await updater.staged();
    if (staged === undefined) {
        throw new Error("expected a staged release");
    }

    return staged;
}

test("retain staging across sessions and reject concurrent or stale activation", async () => {
    // open an independent signed repository and installation
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const { options, target } = repository;

    // stage the complete release under an exclusive updater session
    {
        await using updater = await Updater.open(options);
        expect(await refusal(Updater.open(options))).toEqual([
            "CONFLICT",
            "UPDATE_RUNNING",
            "another Destack update is running",
        ]);

        // serialize operations within the same session
        const checking = updater.check();
        expect(await refusal(updater.check())).toEqual([
            "CONFLICT",
            "UPDATE_RUNNING",
            "an update operation is already running",
        ]);
        const update = await checking;
        expect(update?.release).toEqual(new Release("2026.9.1", target));
        if (update === undefined) {
            throw new Error("expected an available update");
        }

        // retain the complete executable without selecting it
        const download = await update.download();
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
        const staged = await readStaged(updater);
        expect(await refusal(updater.activate(staged))).toEqual([
            "CONFLICT",
            "DOWNGRADE_REFUSED",
            "cannot activate a release older than the running version",
        ]);
        expect(await updater.current()).toBeUndefined();
        expect(await updater.staged()).toEqual(staged);
    }

    // activate only after reopening the persisted staged release
    {
        await using updater = await Updater.open(options);
        const staged = await readStaged(updater);
        const installed = await updater.activate(staged);
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
        expect(await updater.check()).toBeUndefined();
    }
});

test("retry cancelled downloads, refuse a tampered blob, and fetch a tampered download's file again before staging", async () => {
    // open an independent signed repository and installation
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const { options, environment } = repository;

    // reject cancellation before the download starts
    await using updater = await Updater.open(options);
    const update = await checkUpdate(updater);
    const cancelled = new AbortController();
    cancelled.abort(new Error("cancelled download"));
    await expect(update.download({ signal: cancelled.signal })).rejects.toThrow(
        new Error("cancelled download"),
    );

    // cancel after bytes arrive and require the next attempt to download successfully
    const interrupted = new AbortController();
    await expect(
        update.download({
            signal: interrupted.signal,
            onProgress: () => interrupted.abort(new Error("interrupted download")),
        }),
    ).rejects.toThrow(new Error("interrupted download"));
    expect(await updater.current()).toBeUndefined();
    expect(await updater.staged()).toBeUndefined();

    // refuse a blob whose bytes differ from the signed digest, then restore it
    const desktop = await environment.files.readTextFile(
        join(repository.source, "Destack/desktop"),
    );
    const blob = join(
        repository.path,
        "targets/blobs",
        await Digest.of("desktop", environment.crypto),
    );
    const original = await environment.files.readFile(blob);
    await environment.files.writeFile(blob, await gzip("Desktop"));
    const tampered = await refusal(update.download());
    await environment.files.writeFile(blob, original);

    // retry successfully and compare the complete download progress
    const progress: { received: number; total: number }[] = [];
    const download = await update.download({
        onProgress: (value) => progress.push(value),
    });
    const length = download.manifest.files.reduce((sum, file) => sum + file.size, 0);

    // fetch a downloaded file changed on disk again by its digest, staging the signed bytes
    await environment.files.writeTextFile(join(download.directory, "Destack/desktop"), "changed");
    const staged = await updater.stage(download);
    expect({
        tampered,
        progress: progress.at(-1),
        desktop: await environment.files.readTextFile(join(staged.directory, "Destack/desktop")),
        staged: await updater.staged(),
        current: await updater.current(),
        isDownloadKept: await environment.files.exists(download.directory),
    }).toEqual({
        tampered: ["BAD_GATEWAY", "VERIFICATION_FAILED", "blob digest mismatch: Destack/desktop"],
        progress: { received: length, total: length },
        desktop,
        staged,
        current: undefined,
        isDownloadKept: false,
    });
});

test("retain rotated trust across sessions and reject expired or rolled-back metadata", async () => {
    // retain the original timestamp before the repository rotates
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const { options, target, source } = repository;

    const firstTimestamp = await repository.environment.files.readTextFile(
        join(repository.path, "metadata/timestamp.json"),
    );

    // persist initial trust before rotating the repository
    {
        await using updater = await Updater.open(options);
        expect((await updater.check())?.release).toEqual(new Release("2026.9.1", target));
    }

    // rotate both offline and online signing roles
    repository.keys = {
        targets: await SigningKey.generate("ed25519", repository.environment.crypto),
        snapshot: await SigningKey.generate("ed25519", repository.environment.crypto),
        timestamp: await SigningKey.generate("ed25519", repository.environment.crypto),
    };
    const replacement = await Promise.all(
        Array.from({ length: 3 }, () =>
            SigningKey.generate("ed25519", repository.environment.crypto),
        ),
    );
    repository.root = TrustedRoot.create(
        2,
        replacement.map((key) => key.public),
        {
            targets: repository.keys.targets.public,
            snapshot: repository.keys.snapshot.public,
            timestamp: repository.keys.timestamp.public,
        },
        new Date(repository.environment.clock.now() + 365 * 86_400_000).toISOString(),
        repository.environment,
    );
    for (const key of [...rootKeys.slice(0, 2), ...replacement.slice(0, 2)]) {
        await key.sign(repository.root);
    }
    await SignedRepository.create(
        repository.path,
        2,
        repository.root,
        repository.keys,
        [
            {
                kind: "release",
                target,
                version: "2026.9.1",
                commit: COMMIT,
                directory: source,
            },
        ],
        repository.environment,
    );

    // retain rotated trust across independent updater sessions
    {
        await using updater = await Updater.open(options);
        expect((await updater.check())?.release).toEqual(new Release("2026.9.1", target));
    }

    // reject signed expired metadata without selecting or staging another release
    const timestampPath = join(repository.path, "metadata/timestamp.json");
    const timestamp = new Metadata(
        Timestamp.fromJSON({
            ...Metadata.fromJSON(
                MetadataKind.Timestamp,
                parseObject(await repository.environment.files.readTextFile(timestampPath)),
            ).signed.toJSON(),
            version: 3,
            expires: "2000-01-01T00:00:00Z",
        }),
    );
    await repository.keys.timestamp.sign(timestamp);
    await repository.environment.files.writeFile(timestampPath, SignedRepository.encode(timestamp));
    {
        await using updater = await Updater.open(options);
        await expect(updater.check()).rejects.toThrow("Final timestamp.json is expired");
        expect(await updater.current()).toBeUndefined();
    }

    // a valid signature must not permit rollback of persisted metadata
    const rollback = new Metadata(
        Metadata.fromJSON(MetadataKind.Timestamp, parseObject(firstTimestamp)).signed,
    );
    await repository.keys.timestamp.sign(rollback);
    await repository.environment.files.writeFile(timestampPath, SignedRepository.encode(rollback));
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
    await using repository = await TestUpdate.open(
        fixture,
        rootKeys,
        "2026.9.1",
        TestUpdate.world(),
    );
    const { options, target, source } = repository;
    let installed: Awaited<ReturnType<Updater["activate"]>>;

    // establish an active release before publishing inconsistent replacements
    {
        await using updater = await Updater.open(options);
        const update = await checkUpdate(updater);
        installed = await updater.activate(await updater.stage(await update.download()));
    }

    // a signed release with the wrong executable version must leave the installation unchanged
    await SignedRepository.create(
        repository.path,
        3,
        repository.root,
        repository.keys,
        [
            {
                kind: "release",
                target,
                version: "2026.9.2",
                commit: COMMIT,
                directory: source,
            },
        ],
        repository.environment,
    );
    {
        await using updater = await Updater.open(options);
        const update = await checkUpdate(updater);
        const download = await update.download();
        expect(await refusal(updater.stage(download))).toEqual([
            "BAD_GATEWAY",
            "VERIFICATION_FAILED",
            "downloaded CLI version does not match the signed release",
        ]);
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
    }

    // reject changed bytes published under the installed version across sessions
    await repository.environment.files.writeTextFile(
        join(source, "Destack/changed"),
        "changed release contents",
    );
    await SignedRepository.create(
        repository.path,
        4,
        repository.root,
        repository.keys,
        [
            {
                kind: "release",
                target,
                version: "2026.9.1",
                commit: COMMIT,
                directory: source,
            },
        ],
        repository.environment,
    );
    {
        await using updater = await Updater.open(options);
        expect(await refusal(updater.check())).toEqual([
            "CONFLICT",
            "RELEASE_CHANGED",
            "published release changed",
        ]);
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
    }

    // refuse a correctly signed dev build in the feed
    await SignedRepository.create(
        repository.path,
        5,
        repository.root,
        repository.keys,
        [{ kind: "release", target, version: "2026.9.2-dev.1", commit: COMMIT, directory: source }],
        repository.environment,
    );
    {
        await using updater = await Updater.open(options);
        expect(await refusal(updater.check())).toEqual([
            "BAD_GATEWAY",
            "RELEASE_INVALID",
            "the feed publishes dev build 2026.9.2-dev.1",
        ]);
        expect(await updater.current()).toEqual(installed);
        expect(await updater.staged()).toBeUndefined();
    }
});

/** Compress a text with gzip, as the feed stores a blob. */
async function gzip(text: string): Promise<Uint8Array<ArrayBuffer>> {
    const { compression } = testHost();
    const stream = Pipe.through(new Blob([text]).stream(), compression.compress("gzip"));

    return await new Response(stream).bytes();
}
