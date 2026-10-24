import { createHash } from "node:crypto";
import { isDeepStrictEqual } from "node:util";
import { join } from "node:path";
import type { Clock, Environment } from "@destack/environment";
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
import type { SigningKey } from "./key.ts";
import { parseObject } from "./json.ts";
import { TrustedRoot } from "./root.ts";
import { TargetCustom } from "../release/release.ts";
import { UpdateError } from "../error/index.ts";

/** The milliseconds in a day. */
const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The days each online role's metadata stays valid after signing. */
export const LIFETIME_DAYS = { targets: 365, snapshot: 14, timestamp: 14 } as const;

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
    /** Git commit the distribution was built from. */
    commit: string;
    /** Local archive containing the CLI and desktop. */
    archive: string;
    /** The download format, tar.gz for update archives. */
    format?: "dmg" | "exe";
}

/** A signed release repository on disk, as clients download it. */
export const SignedRepository = {
    read: readRepository,
    create: createRepository,
    write: writeMetadata,
    renew: renewMetadata,
    encode,
};

/** Authenticate local metadata against the embedded root before publication. */
async function readRepository(
    directory: string,
    root: Metadata<Root>,
    environment: Environment<"files">,
) {
    // follow consecutive rotations from the root already trusted by installed clients
    const { files } = environment;
    const metadata = join(directory, "metadata");
    const entries = await files.readDirectory(metadata);
    const versions = entries
        .map((entry) => entry.name)
        .filter((file) => /^[1-9]\d*\.root\.json$/u.test(file))
        .map((file) => Number.parseInt(file, 10))
        .toSorted((left, right) => left - right);
    for (const version of versions) {
        const next = Metadata.fromJSON(
            MetadataKind.Root,
            parseObject(await files.readTextFile(join(metadata, `${version}.root.json`))),
        );
        // retain only the trusted root or consecutive authorized replacements
        if (version === root.signed.version) {
            next.verifyDelegate("root", next);
            if (!isDeepStrictEqual(next.signed.toJSON(), root.signed.toJSON())) {
                throw UpdateError.of(
                    "INVALID_METADATA",
                    "published root differs from the trusted root",
                );
            }
        }
        // verify each replacement against the previous quorum
        else {
            TrustedRoot.authenticate(next, root);
            root = next;
        }
    }

    // authenticate the timestamp before using its snapshot reference
    const timestamp = Metadata.fromJSON(
        MetadataKind.Timestamp,
        parseObject(await files.readTextFile(join(metadata, "timestamp.json"))),
    );
    root.verifyDelegate("timestamp", timestamp);
    const snapshotReference = timestamp.signed.snapshotMeta;
    const snapshotBytes = Buffer.from(
        await files.readFile(join(metadata, `${snapshotReference.version}.snapshot.json`)),
    );
    snapshotReference.verify(snapshotBytes);

    // authenticate the snapshot before using its targets reference
    const snapshot = Metadata.fromJSON(
        MetadataKind.Snapshot,
        parseObject(snapshotBytes.toString("utf8")),
    );
    root.verifyDelegate("snapshot", snapshot);
    const targetsReference = snapshot.signed.meta["targets.json"];
    if (!targetsReference || snapshot.signed.version !== snapshotReference.version) {
        throw UpdateError.of("INVALID_METADATA", "snapshot does not match its signed reference");
    }
    const targetsBytes = Buffer.from(
        await files.readFile(join(metadata, `${targetsReference.version}.targets.json`)),
    );
    targetsReference.verify(targetsBytes);

    // reject expired authorization and mismatched signed versions
    const targets = Metadata.fromJSON(
        MetadataKind.Targets,
        parseObject(targetsBytes.toString("utf8")),
    );
    root.verifyDelegate("targets", targets);
    if (targets.signed.version !== targetsReference.version) {
        throw UpdateError.of("INVALID_METADATA", "targets do not match their signed reference");
    }
    if ([root, timestamp, snapshot, targets].some((document) => document.signed.isExpired())) {
        throw UpdateError.of("INVALID_METADATA", "cannot publish expired release metadata");
    }

    return { root, timestamp, snapshot, targets };
}

/** Write a complete signed repository, writing timestamp.json after the files it lists. */
async function createRepository(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: ReleaseKeys,
    distributions: Distribution[],
    environment: Environment<"files">,
): Promise<Metadata<Targets>> {
    if (!Number.isSafeInteger(revision) || revision < 1) {
        throw UpdateError.of("INVALID_METADATA", "invalid metadata revision");
    }
    if (distributions.length === 0) {
        throw UpdateError.of("INVALID_METADATA", "no distributions selected");
    }
    // prepare the signed repository directories
    const { clock, files } = environment;
    const metadata = join(directory, "metadata");
    const archives = join(directory, "targets");
    await files.mkdir(metadata, { recursive: true });
    await files.mkdir(archives, { recursive: true });
    await files.writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));

    // address every archive by its digest and describe its release in signed target metadata
    const targets = new Metadata(
        new Targets({
            version: revision,
            specVersion: "1.0.31",
            expires: expires(LIFETIME_DAYS.targets, clock),
        }),
    );
    for (const distribution of distributions) {
        const path = `${distribution.target}.${distribution.format ?? "tar.gz"}`;
        if (Object.hasOwn(targets.signed.targets, path)) {
            throw UpdateError.of("INVALID_METADATA", `duplicate target: ${path}`);
        }
        const hash = createHash("sha256");
        await using archive = await files.open(distribution.archive, { read: true });
        for await (const bytes of archive.readable()) {
            hash.update(bytes);
        }
        const sha256 = hash.digest("hex");
        const file = await files.stat(distribution.archive);
        const custom = TargetCustom.parse({
            version: distribution.version,
            commit: distribution.commit,
        });
        targets.signed.addTarget(
            new TargetFile({
                path,
                length: file.size,
                hashes: { sha256 },
                unrecognizedFields: { custom },
            }),
        );
        await files.copyFile(distribution.archive, join(archives, `${sha256}.${path}`));
    }

    return await writeMetadata(directory, revision, root, keys, targets, environment);
}

/** Authorize targets, write the matching snapshot and timestamp, and return the signed targets. */
async function writeMetadata(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: ReleaseKeys,
    targets: Metadata<Targets>,
    environment: Environment<"files">,
): Promise<Metadata<Targets>> {
    // authorize a new targets version with the release signing key
    const { clock, files } = environment;
    const metadata = join(directory, "metadata");
    await files.mkdir(metadata, { recursive: true });
    await files.writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));
    targets = new Metadata(
        Targets.fromJSON({
            ...targets.signed.toJSON(),
            version: revision,
            expires: expires(LIFETIME_DAYS.targets, clock),
        }),
    );
    await keys.targets.sign(targets);
    root.verifyDelegate("targets", targets);
    await renewMetadata(directory, revision, root, keys, encode(targets), environment);

    return targets;
}

/** Renew freshness without changing the bytes or signatures of targets metadata. */
async function renewMetadata(
    directory: string,
    revision: number,
    root: Metadata<Root>,
    keys: RenewalKeys,
    targetBytes: Buffer,
    environment: Environment<"files">,
): Promise<void> {
    // verify the existing authorization before signing its continued availability
    const targets = Metadata.fromJSON(
        MetadataKind.Targets,
        parseObject(targetBytes.toString("utf8")),
    );
    root.verifyDelegate("root", root);
    root.verifyDelegate("targets", targets);
    if (root.signed.isExpired() || targets.signed.isExpired()) {
        throw UpdateError.of(
            "INVALID_METADATA",
            "root and targets must be renewed before freshness metadata",
        );
    }
    if (!Number.isSafeInteger(revision) || revision < 1) {
        throw UpdateError.of("INVALID_METADATA", "invalid metadata revision");
    }

    // retain the exact document authenticated by the previous repository
    const { clock, files } = environment;
    const metadata = join(directory, "metadata");
    await files.mkdir(metadata, { recursive: true });
    await files.writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));
    await files.writeFile(join(metadata, `${targets.signed.version}.targets.json`), targetBytes);

    // bind the snapshot and timestamp to the exact metadata bytes
    const snapshot = new Metadata(
        new Snapshot({
            specVersion: "1.0.31",
            version: revision,
            expires: expires(LIFETIME_DAYS.snapshot, clock),
            meta: { "targets.json": describe(targets.signed.version, targetBytes) },
        }),
    );
    await keys.snapshot.sign(snapshot);
    root.verifyDelegate("snapshot", snapshot);
    const snapshotBytes = encode(snapshot);
    await files.writeFile(join(metadata, `${revision}.snapshot.json`), snapshotBytes);
    const timestamp = new Metadata(
        new Timestamp({
            specVersion: "1.0.31",
            version: revision,
            expires: expires(LIFETIME_DAYS.timestamp, clock),
            snapshotMeta: describe(revision, snapshotBytes),
        }),
    );
    await keys.timestamp.sign(timestamp);
    root.verifyDelegate("timestamp", timestamp);
    await files.writeFile(join(metadata, "timestamp.json"), encode(timestamp));
}

/** Encode signed metadata for transport. */
function encode(metadata: Metadata<Root | Targets | Snapshot | Timestamp>): Buffer {
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

/** Expire metadata a bounded number of days after now. */
function expires(days: number, clock: Clock): string {
    return new Date(clock.now() + days * DAY_MILLISECONDS).toISOString();
}
