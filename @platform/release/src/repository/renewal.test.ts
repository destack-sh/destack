import { expect, test } from "@destack/test";
import { Metadata, MetadataKind, MetaFile, Snapshot, Timestamp } from "@tufjs/models";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { SigningKey } from "../key/key.ts";
import { createRoot, verifyRoot } from "../key/root.ts";
import { createRepository, encode, renewMetadata, readRepository } from "./repository.ts";
import { RepositoryRenewal } from "./renewal.ts";
import { RepositoryConfiguration } from "./configuration.ts";

test("recover expired freshness through signed metadata without accepting changed authorization", async () => {
    // serve a disposable signed repository over the same HTTP path used by renewal jobs
    const directory = await mkdtemp(join(tmpdir(), "destack-recovery-"));
    const files = new Map<string, Buffer>();
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        fetch(request) {
            const bytes = files.get(new URL(request.url).pathname);

            return bytes ? new Response(bytes) : new Response(null, { status: 404 });
        },
    });
    try {
        // authorize independent online roles with a disposable offline quorum
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
            new Date(Date.now() + 365 * 86_400_000).toISOString(),
        );
        for (const key of roots.slice(0, 2)) {
            root.sign((bytes) => key.sign(bytes));
        }
        const archive = join(directory, "archive");
        await writeFile(archive, "recovery fixture");
        await createRepository(directory, 1, root, keys, [
            { target: "x86_64-unknown-linux-gnu", version: "2026.9.1", archive },
        ]);
        const targets = await readFile(join(directory, "metadata/1.targets.json"));

        // expire both freshness roles while retaining exact valid target authorization
        const original = JSON.parse(
            await readFile(join(directory, "metadata/1.snapshot.json"), "utf8"),
        );
        const snapshot = new Metadata(
            Snapshot.fromJSON({ ...original.signed, expires: "2000-01-01T00:00:00Z" }),
        );
        snapshot.sign((bytes) => keys.snapshot.sign(bytes));
        const snapshotBytes = encode(snapshot);
        const timestamp = new Metadata(
            new Timestamp({
                version: 1,
                specVersion: "1.0.31",
                expires: "2000-01-01T00:00:00Z",
                snapshotMeta: new MetaFile({
                    version: 1,
                    length: snapshotBytes.length,
                    hashes: { sha256: createHash("sha256").update(snapshotBytes).digest("hex") },
                }),
            }),
        );
        timestamp.sign((bytes) => keys.timestamp.sign(bytes));
        files.set("/metadata/timestamp.json", encode(timestamp));
        files.set("/metadata/1.snapshot.json", snapshotBytes);
        files.set("/metadata/1.targets.json", targets);

        // recover the exact original targets and generate a complete currently valid repository
        const renewal = await RepositoryRenewal.read(server.url, root);
        expect(renewal.targets).toEqual(targets);
        expect(renewal.root.toJSON()).toEqual(root.toJSON());
        await renewMetadata(directory, renewal.revision, renewal.root, keys, renewal.targets);
        const recovered = await readRepository(directory, root);
        expect(recovered.targets.toJSON()).toEqual(
            Metadata.fromJSON(MetadataKind.Targets, JSON.parse(targets.toString())).toJSON(),
        );
        expect(recovered.timestamp.signed.version).toBe(renewal.revision);

        // authenticate an expired bootstrap root without treating it as current authorization
        const expired = Metadata.fromJSON(MetadataKind.Root, {
            ...root.toJSON(),
            signatures: [],
            signed: { ...root.signed.toJSON(), expires: "2000-01-01T00:00:00Z" },
        });
        for (const key of roots.slice(0, 2)) {
            expired.sign((bytes) => key.sign(bytes));
        }
        const bootstrap = join(directory, "bootstrap.json");
        await writeFile(bootstrap, encode(expired));
        const previousRoot = process.env.DESTACK_RELEASE_ROOT;
        const previousFeed = process.env.DESTACK_RELEASE_CHANNEL;
        let initial: typeof root;
        try {
            process.env.DESTACK_RELEASE_ROOT = bootstrap;
            process.env.DESTACK_RELEASE_CHANNEL = "stable";
            initial = await new RepositoryConfiguration().root();
        } finally {
            if (previousRoot === undefined) {
                delete process.env.DESTACK_RELEASE_ROOT;
            } else {
                process.env.DESTACK_RELEASE_ROOT = previousRoot;
            }
            if (previousFeed === undefined) {
                delete process.env.DESTACK_RELEASE_CHANNEL;
            } else {
                process.env.DESTACK_RELEASE_CHANNEL = previousFeed;
            }
        }
        expect(initial.toJSON()).toEqual(expired.toJSON());
        expect(() => verifyRoot(initial)).toThrow("root has expired");
        await expect(RepositoryRenewal.read(server.url, initial)).rejects.toThrow(
            "release root must be renewed by its offline quorum",
        );

        // follow expired intermediate roots only when an authenticated current root follows
        const intermediate = Metadata.fromJSON(MetadataKind.Root, {
            ...expired.toJSON(),
            signatures: [],
            signed: { ...expired.signed.toJSON(), version: 2 },
        });
        const current = Metadata.fromJSON(MetadataKind.Root, {
            ...root.toJSON(),
            signatures: [],
            signed: { ...root.signed.toJSON(), version: 3 },
        });
        for (const key of roots.slice(0, 2)) {
            intermediate.sign((bytes) => key.sign(bytes));
            current.sign((bytes) => key.sign(bytes));
        }
        files.set("/metadata/2.root.json", encode(intermediate));
        await expect(RepositoryRenewal.read(server.url, initial)).rejects.toThrow(
            "release root must be renewed by its offline quorum",
        );
        files.set("/metadata/3.root.json", encode(current));
        const rotated = await RepositoryRenewal.read(server.url, initial);
        expect(rotated.history.map((document) => document.toJSON())).toEqual(
            [initial, intermediate, current].map((document) => document.toJSON()),
        );
        expect(rotated.root.toJSON()).toEqual(current.toJSON());
        expect(rotated.targets).toEqual(targets);

        // retain historical roots during local publication verification
        for (const document of rotated.history) {
            await writeFile(
                join(directory, "metadata", `${document.signed.version}.root.json`),
                encode(document),
            );
        }
        await renewMetadata(directory, rotated.revision, current, keys, rotated.targets);
        const publication = await readRepository(directory, initial);
        expect(publication.root.toJSON()).toEqual(current.toJSON());

        // reject a missing rotation and a changed document even when a later root is valid
        files.delete("/metadata/2.root.json");
        await expect(RepositoryRenewal.read(server.url, initial)).rejects.toThrow(
            "release root must be renewed by its offline quorum",
        );
        files.set("/metadata/2.root.json", encode(intermediate));
        const changed = {
            ...current.toJSON(),
            signed: {
                ...current.signed.toJSON(),
                expires: new Date(Date.now() + 366 * 86_400_000).toISOString(),
            },
        };
        files.set("/metadata/3.root.json", Buffer.from(JSON.stringify(changed)));
        await expect(RepositoryRenewal.read(server.url, initial)).rejects.toThrow(
            "root was signed by 0/2 keys",
        );
        files.set("/metadata/3.root.json", encode(current));

        // refuse a different timestamp signer even though all referenced files remain authentic
        const unauthorized = new Metadata(Timestamp.fromJSON(timestamp.signed.toJSON()));
        const stranger = SigningKey.generate();
        unauthorized.sign((bytes) => stranger.sign(bytes));
        files.set("/metadata/timestamp.json", encode(unauthorized));
        await expect(RepositoryRenewal.read(server.url, root)).rejects.toThrow(
            "timestamp was signed by 0/1 keys",
        );
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
});
