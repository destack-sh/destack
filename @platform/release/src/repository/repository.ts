import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { isDeepStrictEqual } from "node:util";
import { join } from "node:path";
import {
    Metadata,
    MetadataKind,
    MetaFile,
    Root,
    Snapshot,
    TargetFile,
    Targets,
    Timestamp,
} from "@tufjs/models";
import { SigningKey } from "../key/key.ts";
import { authenticateRoot } from "../key/root.ts";
import { writeCatalog } from "./catalog.ts";

/** Keys authorized to publish routine release metadata. */
export interface ReleaseKeys extends RenewalKeys {
    /** Authorize distribution files. */
    targets: SigningKey;
}

/** Keys authorized to refresh existing release metadata. */
export interface RenewalKeys {
    /** Authorize a consistent target metadata version. */
    snapshot: SigningKey;
    /** Authorize the latest snapshot and its expiration. */
    timestamp: SigningKey;
}

/** One distribution to publish in the update repository. */
export interface Distribution {
    /** Operating system and architecture. */
    target: string;
    /** Calendar version reported by the executable. */
    version: string;
    /** Local archive containing the CLI and desktop. */
    archive: string;
    /** Download format; update archives use tar.gz. */
    format?: "dmg" | "exe";
}

/** Authenticate local metadata against the embedded root before publication. */
export async function readRepository(directory: string, root: Metadata<Root>) {
    // follow consecutive rotations from the root already trusted by installed clients
    const metadata = join(directory, "metadata");
    const files = await readdir(metadata);
    const versions = files
        .filter((file) => /^[1-9]\d*\.root\.json$/.test(file))
        .map((file) => Number(file.split(".")[0]))
        .sort((left, right) => left - right);
    for (const version of versions) {
        const next = Metadata.fromJSON(
            MetadataKind.Root,
            JSON.parse(await readFile(join(metadata, `${version}.root.json`), "utf8")),
        );
        // retain only the trusted root or consecutive authorized replacements
        if (version === root.signed.version) {
            next.verifyDelegate("root", next);
            if (!isDeepStrictEqual(next.signed.toJSON(), root.signed.toJSON())) {
                throw new Error("published root differs from the trusted root");
            }
        }
        // verify each replacement against the previous quorum
        else {
            authenticateRoot(next, root);
            root = next;
        }
    }

    // authenticate the timestamp before using its snapshot reference
    const timestamp = Metadata.fromJSON(
        MetadataKind.Timestamp,
        JSON.parse(await readFile(join(metadata, "timestamp.json"), "utf8")),
    );
    root.verifyDelegate("timestamp", timestamp);
    const snapshotReference = timestamp.signed.snapshotMeta;
    const snapshotBytes = await readFile(
        join(metadata, `${snapshotReference.version}.snapshot.json`),
    );
    snapshotReference.verify(snapshotBytes);

    // authenticate the snapshot before using its targets reference
    const snapshot = Metadata.fromJSON(
        MetadataKind.Snapshot,
        JSON.parse(snapshotBytes.toString("utf8")),
    );
    root.verifyDelegate("snapshot", snapshot);
    const targetsReference = snapshot.signed.meta["targets.json"];
    if (!targetsReference || snapshot.signed.version !== snapshotReference.version) {
        throw new Error("snapshot does not match its signed reference");
    }
    const targetsBytes = await readFile(join(metadata, `${targetsReference.version}.targets.json`));
    targetsReference.verify(targetsBytes);

    // reject expired authorization and mismatched signed versions
    const targets = Metadata.fromJSON(
        MetadataKind.Targets,
        JSON.parse(targetsBytes.toString("utf8")),
    );
    root.verifyDelegate("targets", targets);
    if (targets.signed.version !== targetsReference.version) {
        throw new Error("targets do not match their signed reference");
    }
    if ([root, timestamp, snapshot, targets].some((document) => document.signed.isExpired())) {
        throw new Error("cannot publish expired release metadata");
    }

    return { root, timestamp, snapshot, targets };
}

/** Write a complete signed repository; publish timestamp.json after its referenced files. */
export async function createRepository(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: ReleaseKeys,
    distributions: Distribution[],
    url = new URL("https://download.destack.sh/stable/"),
): Promise<void> {
    if (!Number.isSafeInteger(revision) || revision < 1) {
        throw new Error("invalid metadata revision");
    }
    if (distributions.length === 0) {
        throw new Error("no distributions selected");
    }
    // prepare the signed repository directories
    const metadata = join(directory, "metadata");
    const archives = join(directory, "targets");
    await mkdir(metadata, { recursive: true });
    await mkdir(archives, { recursive: true });
    await writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));

    // address every archive by its digest and describe its release in signed target metadata
    const targets = new Metadata(
        new Targets({ version: revision, specVersion: "1.0.31", expires: expires(365) }),
    );
    for (const distribution of distributions) {
        const path = `${distribution.target}.${distribution.format ?? "tar.gz"}`;
        if (Object.hasOwn(targets.signed.targets, path)) {
            throw new Error(`duplicate target: ${path}`);
        }
        const hash = createHash("sha256");
        for await (const bytes of createReadStream(distribution.archive)) {
            hash.update(bytes);
        }
        const sha256 = hash.digest("hex");
        const file = await stat(distribution.archive);
        targets.signed.addTarget(
            new TargetFile({
                path,
                length: file.size,
                hashes: { sha256 },
                unrecognizedFields: { custom: { version: distribution.version } },
            }),
        );
        await copyFile(distribution.archive, join(archives, `${sha256}.${path}`));
    }
    await writeMetadata(directory, revision, root, keys, targets, url);
}

/** Authorize targets and write the corresponding snapshot and timestamp. */
export async function writeMetadata(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: ReleaseKeys,
    targets: Metadata<Targets>,
    url = new URL("https://download.destack.sh/stable/"),
): Promise<void> {
    // authorize a new targets version with the release signing key
    const metadata = join(directory, "metadata");
    await mkdir(metadata, { recursive: true });
    await writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));
    targets = new Metadata(
        Targets.fromJSON({
            ...targets.signed.toJSON(),
            version: revision,
            expires: expires(365),
        }),
    );
    targets.sign((bytes) => keys.targets.sign(bytes));
    root.verifyDelegate("targets", targets);
    await renewMetadata(directory, revision, root, keys, encode(targets));
    await writeCatalog(directory, targets, url);
}

/** Renew freshness without changing the bytes or signatures of targets metadata. */
export async function renewMetadata(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: RenewalKeys,
    targetBytes: Buffer,
): Promise<void> {
    // verify the existing authorization before signing its continued availability
    const targets = Metadata.fromJSON(
        MetadataKind.Targets,
        JSON.parse(targetBytes.toString("utf8")),
    );
    root.verifyDelegate("root", root);
    root.verifyDelegate("targets", targets);
    if (root.signed.isExpired() || targets.signed.isExpired()) {
        throw new Error("root and targets must be renewed before freshness metadata");
    }
    if (!Number.isSafeInteger(revision) || revision < 1) {
        throw new Error("invalid metadata revision");
    }

    // retain the exact document authenticated by the previous repository
    const metadata = join(directory, "metadata");
    await mkdir(metadata, { recursive: true });
    await writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));
    await writeFile(join(metadata, `${targets.signed.version}.targets.json`), targetBytes);

    // bind the snapshot and timestamp to the exact metadata bytes
    const snapshot = new Metadata(
        new Snapshot({
            specVersion: "1.0.31",
            version: revision,
            expires: expires(7),
            meta: { "targets.json": describe(targets.signed.version, targetBytes) },
        }),
    );
    snapshot.sign((bytes) => keys.snapshot.sign(bytes));
    root.verifyDelegate("snapshot", snapshot);
    const snapshotBytes = encode(snapshot);
    await writeFile(join(metadata, `${revision}.snapshot.json`), snapshotBytes);
    const timestamp = new Metadata(
        new Timestamp({
            specVersion: "1.0.31",
            version: revision,
            expires: expires(2),
            snapshotMeta: describe(revision, snapshotBytes),
        }),
    );
    timestamp.sign((bytes) => keys.timestamp.sign(bytes));
    root.verifyDelegate("timestamp", timestamp);
    await writeFile(join(metadata, "timestamp.json"), encode(timestamp));
}

/** Encode signed metadata for transport. */
export function encode(metadata: Metadata<Root | Targets | Snapshot | Timestamp>): Buffer {
    return Buffer.from(JSON.stringify(metadata.toJSON()) + "\n");
}

/** Describe one exact metadata document. */
function describe(version: number, bytes: Buffer): MetaFile {
    return new MetaFile({ version, length: bytes.length, hashes: { sha256: digest(bytes) } });
}

/** Calculate a SHA-256 content digest. */
function digest(bytes: Buffer): string {
    return createHash("sha256").update(bytes).digest("hex");
}

/** Expire metadata after a bounded number of days. */
function expires(days: number): string {
    return new Date(Date.now() + days * 86_400_000).toISOString();
}
