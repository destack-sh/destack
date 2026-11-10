import { createHash } from "node:crypto";
import { isDeepStrictEqual } from "node:util";
import { join } from "node:path";
import { type Clock, type Environment, Pipe } from "@destack/environment";
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
import { Manifest } from "../manifest/manifest.ts";

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

/** One distribution to publish: a release's or a toolchain's files, or an installer. */
export type Distribution = {
    /** Operating system and architecture, or the installer's name. */
    target: string;
    /** Calendar version reported by the executable. */
    version: string;
    /** Git commit the distribution was built from. */
    commit: string;
} & (
    | {
          /** A release, published as its signed manifest and its files' blobs. */
          kind: "release";
          /** The directory with the release's files, as an installation keeps them. */
          directory: string;
      }
    | {
          /** A toolchain, published as its files' blobs, which the manifest in a release lists. */
          kind: "toolchain";
          /** The directory with the toolchain's files, whose modes the manifest records. */
          directory: string;
          /** The manifest a release's resources list. */
          manifest: Manifest;
      }
    | {
          /** An installer people download. */
          kind: "installer";
          /** The installer's file. */
          archive: string;
          /** The installer's format. */
          format: "tar.gz" | "dmg" | "exe";
      }
);

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
                    "METADATA_INVALID",
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
        throw UpdateError.of("METADATA_INVALID", "snapshot does not match its signed reference");
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
        throw UpdateError.of("METADATA_INVALID", "targets do not match their signed reference");
    }
    if ([root, timestamp, snapshot, targets].some((document) => document.signed.isExpired())) {
        throw UpdateError.of("METADATA_INVALID", "cannot publish expired release metadata");
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
        throw UpdateError.of("METADATA_INVALID", "invalid metadata revision");
    }
    if (distributions.length === 0) {
        throw UpdateError.of("METADATA_INVALID", "no distributions selected");
    }
    // prepare the signed repository directories
    const { clock, files } = environment;
    const metadata = join(directory, "metadata");
    const archives = join(directory, "targets");
    await files.mkdir(metadata, { recursive: true });
    await files.mkdir(archives, { recursive: true });
    await files.writeFile(join(metadata, `${root.signed.version}.root.json`), encode(root));

    // sign one target per distribution
    const targets = new Metadata(
        new Targets({
            version: revision,
            specVersion: "1.0.31",
            expires: expires(LIFETIME_DAYS.targets, clock),
        }),
    );
    for (const distribution of distributions) {
        // publish a toolchain's blobs alone, as its manifest ships inside the signed release
        if (distribution.kind === "toolchain") {
            await writeToolchain(distribution, archives, environment);
            continue;
        }

        // publish a release's manifest and blobs, else the installer itself
        const format = distribution.kind === "release" ? "json" : distribution.format;
        const path = `${distribution.target}.${format}`;
        if (Object.hasOwn(targets.signed.targets, path)) {
            throw UpdateError.of("METADATA_INVALID", `duplicate target: ${path}`);
        }
        const source =
            distribution.kind === "release"
                ? await writeManifest(
                      await Manifest.read(distribution.directory, environment),
                      distribution.directory,
                      archives,
                      path,
                      environment,
                  )
                : distribution.archive;

        // address the target by its digest and describe its release in signed metadata
        const hash = createHash("sha256");
        await using archive = await files.open(source, { read: true });
        for await (const bytes of archive.readable()) {
            hash.update(bytes);
        }
        const sha256 = hash.digest("hex");
        const file = await files.stat(source);
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

        // move a release's manifest to its address, and copy an installer
        const destination = join(archives, `${sha256}.${path}`);
        if (distribution.kind === "release") {
            await files.rename(source, destination);
        } else {
            await files.copyFile(source, destination);
        }
    }

    return await writeMetadata(directory, revision, root, keys, targets, environment);
}

/** Write a toolchain's blobs, refusing files other than its manifest lists. */
async function writeToolchain(
    toolchain: Extract<Distribution, { kind: "toolchain" }>,
    targets: string,
    environment: Environment<"files">,
): Promise<void> {
    // compare the files by path, digest and size, as the manifest's modes are the ones published
    const read = await Manifest.read(toolchain.directory, environment);
    if (describeFiles(read) !== describeFiles(toolchain.manifest)) {
        throw UpdateError.of(
            "METADATA_INVALID",
            `the toolchain's files differ from its manifest: ${toolchain.directory}`,
        );
    }

    await writeBlobs(toolchain.manifest, toolchain.directory, targets, environment);
}

/** Describe a manifest's files by path, digest and size, and its links, leaving the modes out. */
function describeFiles(manifest: Manifest): string {
    const files = manifest.files.map((file) => ({
        path: file.path,
        digest: file.digest,
        size: file.size,
    }));

    return JSON.stringify({ files, links: manifest.links });
}

/** Write a manifest's blobs from a directory, each file gzipped at its digest, and its file. */
async function writeManifest(
    manifest: Manifest,
    directory: string,
    targets: string,
    name: string,
    environment: Environment<"files">,
): Promise<string> {
    // write the files' blobs, then the manifest under its name, until the target's digest addresses it
    await writeBlobs(manifest, directory, targets, environment);
    const path = join(targets, name);
    await environment.files.writeTextFile(path, manifest.encode());

    return path;
}

/** Write a manifest's files from a directory as blobs, each gzipped at its digest. */
async function writeBlobs(
    manifest: Manifest,
    directory: string,
    targets: string,
    environment: Environment<"files">,
): Promise<void> {
    // write each file's blob once, compressed beside its address and renamed in when complete
    const { files } = environment;
    const blobs = join(targets, "blobs");
    await files.mkdir(blobs, { recursive: true });
    for (const file of manifest.files) {
        const blob = join(blobs, file.digest);
        if (!(await files.exists(blob))) {
            const staging = `${blob}.partial`;
            await files.remove(staging, { force: true });
            {
                await using source = await files.open(join(directory, file.path), { read: true });
                await using written = await files.open(staging, { write: true, createNew: true });
                const compressed = Pipe.through(
                    source.readable(),
                    environment.compression.compress("gzip"),
                );
                await Pipe.to(compressed, written.writable());
            }
            await files.rename(staging, blob);
        }
    }
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
            "METADATA_INVALID",
            "root and targets must be renewed before freshness metadata",
        );
    }
    if (!Number.isSafeInteger(revision) || revision < 1) {
        throw UpdateError.of("METADATA_INVALID", "invalid metadata revision");
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
