import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Metadata, MetadataKind, MetaFile, Snapshot, Timestamp } from "@tufjs/models";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { RepositoryRenewal } from "./renewal.ts";
import { RepositoryConfiguration } from "./configuration.ts";
import { MetadataDocument } from "./document.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";
import { COMMIT } from "@destack/update/test";

/** The real host the repository is served from and read on. */
const HOST = testHost();

test("recover expired freshness through signed metadata without accepting changed authorization", async () => {
    // serve a disposable signed repository over the same HTTP path used by renewal jobs
    const directory = await HOST.files.makeTemporaryDirectory("destack-recovery-");
    const files = new Map<string, Buffer>();
    const server = HOST.server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch(request) {
            const bytes = files.get(new URL(request.url).pathname);

            return bytes
                ? new Response(new Uint8Array(bytes))
                : new Response(null, { status: 404 });
        },
    });
    try {
        // authorize independent online roles with a disposable offline quorum
        const roots = await Promise.all(
            Array.from({ length: 3 }, () => SigningKey.generate("ed25519", HOST.crypto)),
        );
        const keys = {
            targets: await SigningKey.generate("ed25519", HOST.crypto),
            snapshot: await SigningKey.generate("ed25519", HOST.crypto),
            timestamp: await SigningKey.generate("ed25519", HOST.crypto),
        };
        const root = TrustedRoot.create(
            1,
            roots.map((key) => key.public),
            {
                targets: keys.targets.public,
                snapshot: keys.snapshot.public,
                timestamp: keys.timestamp.public,
            },
            new Date(HOST.clock.now() + 365 * 86_400_000).toISOString(),
            HOST,
        );
        for (const key of roots.slice(0, 2)) {
            await key.sign(root);
        }

        // read no published state before the first publication
        expect(await RepositoryRenewal.read(server.url, root, HOST)).toBeUndefined();
        const archive = join(directory, "archive");
        await HOST.files.writeTextFile(archive, "recovery fixture");
        await SignedRepository.create(
            directory,
            1,
            root,
            keys,
            [{ target: "x86_64-unknown-linux-gnu", version: "2026.9.1", commit: COMMIT, archive }],
            HOST,
        );
        const targets = Buffer.from(
            await HOST.files.readFile(join(directory, "metadata/1.targets.json")),
        );

        // expire both freshness roles while retaining exact valid target authorization
        const original = Metadata.fromJSON(
            MetadataKind.Snapshot,
            MetadataDocument.parse(
                await HOST.files.readTextFile(join(directory, "metadata/1.snapshot.json")),
            ),
        );
        const snapshot = new Metadata(
            Snapshot.fromJSON({ ...original.signed.toJSON(), expires: "2000-01-01T00:00:00Z" }),
        );
        await keys.snapshot.sign(snapshot);
        const snapshotBytes = SignedRepository.encode(snapshot);
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
        await keys.timestamp.sign(timestamp);
        files.set("/metadata/timestamp.json", SignedRepository.encode(timestamp));
        files.set("/metadata/1.snapshot.json", snapshotBytes);
        files.set("/metadata/1.targets.json", targets);

        // recover the exact original targets and generate a complete currently valid repository
        const renewal = present(
            await RepositoryRenewal.read(server.url, root, HOST),
            "the published repository",
        );
        expect(renewal.targets).toEqual(targets);
        expect(renewal.root.toJSON()).toEqual(root.toJSON());

        // report the commit the published targets were built from and the version they report
        expect(renewal.isBuiltFrom(COMMIT)).toBe(true);
        expect(renewal.isBuiltFrom("0".repeat(40))).toBe(false);
        expect(renewal.version).toBe("2026.9.1");
        await SignedRepository.renew(
            directory,
            renewal.revision,
            renewal.root,
            keys,
            renewal.targets,
            HOST,
        );
        const recovered = await SignedRepository.read(directory, root, HOST);
        expect(recovered.targets.toJSON()).toEqual(
            Metadata.fromJSON(
                MetadataKind.Targets,
                MetadataDocument.parse(targets.toString()),
            ).toJSON(),
        );
        expect(recovered.timestamp.signed.version).toBe(renewal.revision);

        // authenticate an expired bootstrap root without treating it as current authorization
        const expired = Metadata.fromJSON(MetadataKind.Root, {
            ...root.toJSON(),
            signatures: [],
            signed: { ...root.signed.toJSON(), expires: "2000-01-01T00:00:00Z" },
        });
        for (const key of roots.slice(0, 2)) {
            await key.sign(expired);
        }
        const initial = new RepositoryConfiguration(HOST, { root: expired }).root();
        expect(initial.toJSON()).toEqual(expired.toJSON());
        expect(() => TrustedRoot.verify(initial)).toThrow("root has expired");
        await expect(RepositoryRenewal.read(server.url, initial, HOST)).rejects.toThrow(
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
            await key.sign(intermediate);
            await key.sign(current);
        }
        files.set("/metadata/2.root.json", SignedRepository.encode(intermediate));
        await expect(RepositoryRenewal.read(server.url, initial, HOST)).rejects.toThrow(
            "release root must be renewed by its offline quorum",
        );
        files.set("/metadata/3.root.json", SignedRepository.encode(current));
        const rotated = present(
            await RepositoryRenewal.read(server.url, initial, HOST),
            "the published repository",
        );
        expect(rotated.history.map((document) => document.toJSON())).toEqual(
            [initial, intermediate, current].map((document) => document.toJSON()),
        );
        expect(rotated.root.toJSON()).toEqual(current.toJSON());
        expect(rotated.targets).toEqual(targets);

        // retain historical roots during local publication verification
        for (const document of rotated.history) {
            await HOST.files.writeFile(
                join(directory, "metadata", `${document.signed.version}.root.json`),
                SignedRepository.encode(document),
            );
        }
        await SignedRepository.renew(
            directory,
            rotated.revision,
            current,
            keys,
            rotated.targets,
            HOST,
        );
        const publication = await SignedRepository.read(directory, initial, HOST);
        expect(publication.root.toJSON()).toEqual(current.toJSON());

        // reject a missing rotation and a changed document even when a later root is valid
        files.delete("/metadata/2.root.json");
        await expect(RepositoryRenewal.read(server.url, initial, HOST)).rejects.toThrow(
            "release root must be renewed by its offline quorum",
        );
        files.set("/metadata/2.root.json", SignedRepository.encode(intermediate));
        const changed = {
            ...current.toJSON(),
            signed: {
                ...current.signed.toJSON(),
                expires: new Date(HOST.clock.now() + 366 * 86_400_000).toISOString(),
            },
        };
        files.set("/metadata/3.root.json", Buffer.from(JSON.stringify(changed)));
        await expect(RepositoryRenewal.read(server.url, initial, HOST)).rejects.toThrow(
            "root was signed by 0/2 keys",
        );
        files.set("/metadata/3.root.json", SignedRepository.encode(current));

        // refuse a different timestamp signer even though all referenced files remain authentic
        const unauthorized = new Metadata(Timestamp.fromJSON(timestamp.signed.toJSON()));
        const stranger = await SigningKey.generate("ed25519", HOST.crypto);
        await stranger.sign(unauthorized);
        files.set("/metadata/timestamp.json", SignedRepository.encode(unauthorized));
        await expect(RepositoryRenewal.read(server.url, root, HOST)).rejects.toThrow(
            "timestamp was signed by 0/1 keys",
        );
    } finally {
        await server.stop(true);
        await HOST.files.remove(directory, { recursive: true, force: true });
    }
});
