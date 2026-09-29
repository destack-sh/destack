import { Metadata, MetadataKind, type Root, type Timestamp } from "@tufjs/models";
import { authenticateRoot } from "../key/root.ts";

/** Maximum release metadata document size, in bytes. */
const METADATA_SIZE = 1024 * 1024;

/** Authenticated publication state used to renew expired freshness metadata. */
export class RepositoryRenewal {
    /** Consecutive roots authenticated from the embedded bootstrap document. */
    readonly history: Metadata<Root>[];
    /** Exact targets bytes retained by freshness renewal. */
    readonly targets: Buffer;
    /** Authenticated current timestamp, including its publication revision. */
    readonly timestamp: Metadata<Timestamp>;
    /** Revision greater than the previously signed timestamp and snapshot. */
    readonly revision: number;

    /** Retain authenticated documents and the next publication revision. */
    constructor(
        history: Metadata<Root>[],
        targets: Buffer,
        timestamp: Metadata<Timestamp>,
        revision: number,
    ) {
        // retain the authenticated publication and next revision together
        this.history = history;
        this.targets = targets;
        this.timestamp = timestamp;
        this.revision = revision;
    }

    /** Latest root authenticated by every intervening quorum. */
    get root(): Metadata<Root> {
        return this.history[this.history.length - 1]!;
    }

    /** Authenticate publication state while allowing expired freshness documents. */
    static async read(url: URL, root: Metadata<Root>): Promise<RepositoryRenewal> {
        // follow consecutive quorum-authorized rotations before accepting online signatures
        const history = await readRootHistory(url, root);
        root = history[history.length - 1]!;

        // authenticate the timestamp and its exact snapshot bytes after freshness expires
        const timestampBytes = await readDocument(url, "timestamp.json");
        const timestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            JSON.parse(timestampBytes.toString()),
        );
        root.verifyDelegate("timestamp", timestamp);
        const snapshotReference = timestamp.signed.snapshotMeta;
        const snapshotBytes = await readDocument(url, `${snapshotReference.version}.snapshot.json`);
        snapshotReference.verify(snapshotBytes);
        const snapshot = Metadata.fromJSON(
            MetadataKind.Snapshot,
            JSON.parse(snapshotBytes.toString()),
        );
        root.verifyDelegate("snapshot", snapshot);
        if (snapshot.signed.version !== snapshotReference.version) {
            throw new Error("snapshot does not match its signed reference");
        }

        // retain exact target authorization for the publication service's storage comparison
        const reference = snapshot.signed.meta["targets.json"];
        if (!reference || Object.keys(snapshot.signed.meta).length !== 1) {
            throw new Error("release snapshot must reference only targets.json");
        }
        const targets = await readDocument(url, `${reference.version}.targets.json`);
        reference.verify(targets);
        const authorization = Metadata.fromJSON(
            MetadataKind.Targets,
            JSON.parse(targets.toString()),
        );
        root.verifyDelegate("targets", authorization);
        if (authorization.signed.version !== reference.version) {
            throw new Error("targets do not match their signed reference");
        }

        // choose a monotonic safe integer revision for the next publication
        const revision = Math.max(
            Date.now(),
            timestamp.signed.version + 1,
            snapshot.signed.version + 1,
        );
        if (!Number.isSafeInteger(revision)) {
            throw new Error("release metadata revision exceeds the supported range");
        }

        return new RepositoryRenewal(history, targets, timestamp, revision);
    }
}

/** Follow the complete authenticated root history and require an unexpired final root. */
export async function readRootHistory(url: URL, root: Metadata<Root>): Promise<Metadata<Root>[]> {
    // allow expired historical authorization only while following consecutive signed replacements
    authenticateRoot(root);
    const history = [root];
    for (;;) {
        const bytes = await readDocument(url, `${root.signed.version + 1}.root.json`, true);
        if (!bytes) {
            break;
        }
        const next = Metadata.fromJSON(MetadataKind.Root, JSON.parse(bytes.toString()));
        authenticateRoot(next, root);
        history.push(next);
        root = next;
    }
    if (root.signed.isExpired()) {
        throw new Error("release root must be renewed by its offline quorum");
    }

    return history;
}

/** Fetch a required metadata document. */
async function readDocument(url: URL, name: string): Promise<Buffer>;
/** Fetch the next root, allowing its absence. */
async function readDocument(url: URL, name: string, optional: true): Promise<Buffer | undefined>;
/** Read bounded signed bytes from the selected origin. */
async function readDocument(url: URL, name: string, optional = false): Promise<Buffer | undefined> {
    // read without redirects or intermediary caches and bound the request duration
    const response = await fetch(new URL(`metadata/${name}`, url), {
        cache: "no-store",
        redirect: "error",
        signal: AbortSignal.timeout(15000),
    });
    if (optional && response.status === 404) {
        await response.body?.cancel();

        return undefined;
    }
    if (!response.ok || !response.body) {
        await response.body?.cancel();
        throw new Error(`cannot read release metadata ${name}: ${response.status}`);
    }

    // consume bounded chunks and cancel oversized responses through iterator cleanup
    const buffers: Uint8Array[] = [];
    let size = 0;
    for await (const bytes of response.body) {
        size += bytes.length;
        if (size > METADATA_SIZE) {
            throw new Error(`oversized release metadata: ${name}`);
        }
        buffers.push(bytes);
    }

    return Buffer.concat(buffers);
}
