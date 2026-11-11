import type { Environment } from "@destack/environment";
import { join } from "node:path";
import { Readable } from "node:stream";
import {
    Metadata,
    MetadataKind,
    type TargetFile,
    type Targets,
    type Timestamp,
} from "@tufjs/models";
import { Version } from "@destack/schema";
import { TargetCustom } from "@destack/update/release";
import { RepositoryConfiguration } from "./index.ts";
import { RepositoryRenewal } from "./renewal.ts";
import { R2ReleaseBucket } from "./bucket.ts";
import { SignedRepository } from "@destack/update/publish";
import { Manifest } from "@destack/update";
import { MetadataDocument } from "./document.ts";
import { PUBLIC_PATH } from "./layout.ts";

/** The signed timestamp whose replacement makes a publication current. */
const TIMESTAMP = "metadata/timestamp.json";

/** The files a publication replaces in place, the timestamp first. */
const MUTABLE_FILES = [TIMESTAMP, "downloads.json", "install"];

/** The directory of the blobs a release's manifest lists, each at its digest. */
const BLOBS = "targets/blobs/";

/** The uploads a publication runs at once. */
const UPLOAD_CONCURRENCY = 16;

/** The media type of each publication file suffix. */
const CONTENT_TYPES: Readonly<Record<string, string>> = {
    ".json": "application/json",
    ".tar.gz": "application/gzip",
    ".dmg": "application/x-apple-diskimage",
};

/** Upload immutable files before publishing the timestamp that makes them current. */
export async function publish(
    directory: string,
    configuration: RepositoryConfiguration,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // select the destination before accessing its publication credential
    const bucket = new R2ReleaseBucket(environment);
    const files = await list(directory, environment);
    const unexpected = files.find((path) => !PUBLIC_PATH.test(path));
    if (unexpected !== undefined) {
        throw new Error(`unexpected file in public release repository: '${unexpected}'`);
    }
    if (!files.includes(TIMESTAMP)) {
        throw new Error("missing signed timestamp");
    }

    // authenticate the signed repository, each target it lists and each manifest's blobs
    const verified = await SignedRepository.read(directory, configuration.root(), environment);
    const referenced = new Set([
        `metadata/${verified.snapshot.signed.version}.snapshot.json`,
        `metadata/${verified.targets.signed.version}.targets.json`,
    ]);
    for (const target of Object.values(verified.targets.signed.targets)) {
        const path = targetPath(target);
        referenced.add(path);
        if (files.includes(path)) {
            await using file = await environment.files.open(join(directory, path), { read: true });
            await target.verify(Readable.from(file.readable()));
        }

        // reference the blobs of a release's manifest
        if (target.path.endsWith(".json") && files.includes(path)) {
            const manifest = Manifest.parse(
                await environment.files.readTextFile(join(directory, path)),
            );
            for (const file of manifest.files) {
                referenced.add(`${BLOBS}${file.digest}`);
            }
        }
    }

    // continue the published repository or require every file of the first one
    const revision = await requirePublishable(
        configuration,
        bucket,
        verified.timestamp,
        verified.targets,
        files,
        referenced,
    );

    // upload the files under the read timestamp revision
    await upload(bucket, directory, files, referenced, revision);
}

/** Require the files to continue the published repository or form a complete first publication. */
async function requirePublishable(
    configuration: RepositoryConfiguration,
    bucket: R2ReleaseBucket,
    timestamp: Metadata<Timestamp>,
    targets: Metadata<Targets>,
    files: string[],
    referenced: ReadonlySet<string>,
): Promise<string | undefined> {
    // read the current timestamp and its revision
    const { environment } = bucket;
    const current = await bucket.get(TIMESTAMP);
    const revision = await readRevision(current);

    // continue the published repository
    if (revision !== undefined) {
        const stored = Metadata.fromJSON(
            MetadataKind.Timestamp,
            MetadataDocument.parse(await current.text()),
        );
        await requireContinuation(configuration, stored, timestamp, targets, files, environment);
    }
    // require every file of the first publication
    else {
        const missing = [...referenced].find((path) => !files.includes(path));
        if (missing !== undefined) {
            throw new Error(`release file is missing: '${missing}'`);
        }
    }

    return revision;
}

/** Upload the immutable files and replace each mutable file at its read revision. */
async function upload(
    bucket: R2ReleaseBucket,
    directory: string,
    files: string[],
    referenced: ReadonlySet<string>,
    revision: string | undefined,
): Promise<void> {
    // read the revisions of the mutable files before any write
    const revisions = new Map([[TIMESTAMP, revision]]);
    for (const path of MUTABLE_FILES.filter((file) => file !== TIMESTAMP && files.includes(file))) {
        revisions.set(path, await bucket.revision(path));
    }

    // upload immutable objects a few at a time, skipping blobs the feed has at their digest
    const pending = files.filter((file) => isImmutable(file, referenced));
    const uploader = async (): Promise<void> => {
        for (let path = pending.shift(); path !== undefined; path = pending.shift()) {
            const isStored = path.startsWith(BLOBS) && (await bucket.revision(path)) !== undefined;
            if (!isStored) {
                await bucket.put(path, join(directory, path), contentType(path), true);
            }
        }
    };
    await Promise.all(Array.from({ length: UPLOAD_CONCURRENCY }, uploader));

    // replace the mutable files at their read revisions
    for (const path of MUTABLE_FILES.filter((file) => files.includes(file))) {
        await bucket.put(
            path,
            join(directory, path),
            contentType(path),
            false,
            revisions.get(path),
        );
    }
}

/** Read the current timestamp's object revision, absent before the first publication. */
async function readRevision(current: Response): Promise<string | undefined> {
    // treat a missing timestamp as the first publication
    if (current.status === 404) {
        await current.body?.cancel();

        return undefined;
    }

    // require a readable timestamp with an object revision
    const revision = current.headers.get("etag");
    if (!current.ok) {
        await current.body?.cancel();
        throw new Error(`cannot read current release timestamp: ${current.status}`);
    }
    if (revision === null || revision === "") {
        await current.body?.cancel();
        throw new Error("current release timestamp has no object revision");
    }

    return revision;
}

/** Require a publication to continue the published repository without rollback or removal. */
async function requireContinuation(
    configuration: RepositoryConfiguration,
    stored: Metadata<Timestamp>,
    next: Metadata<Timestamp>,
    targets: Metadata<Targets>,
    files: string[],
    environment: Environment,
): Promise<void> {
    // require the public metadata to match authoritative storage
    const renewal = await RepositoryRenewal.read(
        configuration.url,
        configuration.root(),
        environment,
    );
    if (renewal === undefined || !isSameDocument(stored, renewal.timestamp)) {
        throw new Error("public metadata differs from current storage; retry publication");
    }

    // resume an interrupted publication only when its signed timestamp is unchanged
    const previous = renewal.timestamp;
    const isRollback = previous.signed.version > next.signed.version;
    const isConflict =
        previous.signed.version === next.signed.version && !isSameDocument(previous, next);
    if (isRollback || isConflict) {
        throw new Error("metadata revision must increase or retain the exact signed timestamp");
    }

    // keep every published platform at the same or a later version
    const published = Metadata.fromJSON(
        MetadataKind.Targets,
        MetadataDocument.parse(renewal.targets.toString()),
    );
    for (const [path, old] of Object.entries(published.signed.targets)) {
        requireSuccessor(path, old, targets.signed.targets[path]);
    }

    // require every omitted archive to retain its authenticated published identity
    for (const target of Object.values(targets.signed.targets)) {
        const path = targetPath(target);
        const existing = published.signed.targets[target.path];
        if (!files.includes(path) && (existing === undefined || !isSameFile(existing, target))) {
            throw new Error(`release archive is missing: '${path}'`);
        }
    }
}

/** Require a platform's replacement target to raise its version, or keep it unchanged. */
function requireSuccessor(
    path: string,
    old: TargetFile,
    replacement: TargetFile | undefined,
): void {
    if (replacement === undefined) {
        throw new Error(`refusing to remove a published platform: '${path}'`);
    }
    const oldVersion = TargetCustom.parse(old.custom).version;
    const newVersion = TargetCustom.parse(replacement.custom).version;

    // refuse downgrades and different bytes under a published version
    const comparison = Version.compare(newVersion, oldVersion);
    if (comparison < 0) {
        throw new Error(`refusing to downgrade '${path}'`);
    }
    if (comparison === 0 && !isSameFile(old, replacement)) {
        throw new Error(`published release is immutable: ${path} '${oldVersion}'`);
    }
}

/** Report whether two signed documents are identical. */
function isSameDocument(first: Metadata<Timestamp>, second: Metadata<Timestamp>): boolean {
    return JSON.stringify(first.toJSON()) === JSON.stringify(second.toJSON());
}

/** Report whether two targets describe the same bytes. */
function isSameFile(first: TargetFile, second: TargetFile): boolean {
    return first.hashes["sha256"] === second.hashes["sha256"] && first.length === second.length;
}

/** Report whether a file is an immutable object: referenced metadata, a target or a root. */
function isImmutable(path: string, referenced: ReadonlySet<string>): boolean {
    return (
        !MUTABLE_FILES.includes(path) &&
        (referenced.has(path) || /^metadata\/\d+\.root\.json$/u.test(path))
    );
}

/** Name a target's content-addressed path in the repository. */
function targetPath(target: TargetFile): string {
    return `targets/${target.hashes["sha256"]}.${target.path}`;
}

/** Select the media type of an explicitly supported publication file. */
function contentType(path: string): string {
    // serve blobs as their gzip bytes, which updaters decompress
    if (path.startsWith(BLOBS)) {
        return "application/gzip";
    }

    // keep installer downloads distinct from compressed archives
    const match = Object.entries(CONTENT_TYPES).find(([suffix]) => path.endsWith(suffix));
    if (match !== undefined) {
        return match[1];
    }
    if (path === "install") {
        return "text/plain; charset=utf-8";
    }

    // reject files outside the public release formats
    throw new Error(`unsupported publication media type: '${path}'`);
}

/** List files relative to the public repository directory. */
async function list(directory: string, environment: Environment<"files">): Promise<string[]> {
    // list every entry below the directory, refusing anything but files and directories
    const files: string[] = [];
    for (const entry of await environment.files.readDirectory(directory, { recursive: true })) {
        if (entry.kind === "file") {
            files.push(entry.name);
        } else if (entry.kind !== "directory") {
            throw new Error(`unsupported release file: '${entry.name}'`);
        }
    }

    return files.toSorted();
}
