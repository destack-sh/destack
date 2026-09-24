import type { R2Bucket, R2ObjectBody } from "@cloudflare/workers-types";
import { Metadata, MetadataKind, type Root } from "@tufjs/models";
import { Renewal } from "./renewal.ts";

/** Maximum signed metadata document size, in bytes. */
const METADATA_SIZE = 1024 * 1024;

/** Publication operations restricted to one release repository. */
export class Publication {
    /** Bucket containing this repository's public objects. */
    readonly bucket: R2Bucket;
    /** Object prefix matching the public release URL. */
    readonly prefix: string;
    /** Embedded initial root authorized by the offline quorum. */
    readonly root: Metadata<Root>;

    /** Select a repository without loading any signing credential. */
    constructor(bucket: R2Bucket, prefix: string, root: Metadata<Root>) {
        this.bucket = bucket;
        this.prefix = prefix;
        this.root = root;
    }

    /** Authenticate and publish freshness while retaining current targets and all artifacts. */
    async renew(snapshot: Buffer, timestamp: Buffer): Promise<void> {
        // read the authoritative timestamp and its exact metadata references directly from storage
        const current = await this.read("metadata/timestamp.json");
        const currentBytes = Buffer.from(await current.arrayBuffer());
        const previous = Metadata.fromJSON(
            MetadataKind.Timestamp,
            JSON.parse(currentBytes.toString()),
        );
        const snapshotObject = await this.read(
            `metadata/${previous.signed.snapshotMeta.version}.snapshot.json`,
        );
        const snapshotBytes = Buffer.from(await snapshotObject.arrayBuffer());
        previous.signed.snapshotMeta.verify(snapshotBytes);
        const stored = Metadata.fromJSON(
            MetadataKind.Snapshot,
            JSON.parse(snapshotBytes.toString()),
        );
        const reference = stored.signed.meta["targets.json"];
        if (!reference || stored.signed.version !== previous.signed.snapshotMeta.version) {
            throw new Error("stored snapshot does not match the current timestamp");
        }

        // authenticate root rotations and the exact targets retained by the current snapshot
        const targets = await this.read(`metadata/${reference.version}.targets.json`);
        const targetBytes = Buffer.from(await targets.arrayBuffer());
        reference.verify(targetBytes);
        const authorization = Metadata.fromJSON(
            MetadataKind.Targets,
            JSON.parse(targetBytes.toString()),
        );
        if (authorization.signed.version !== reference.version) {
            throw new Error("stored targets do not match the current snapshot");
        }
        const root = await this.rotate();
        root.verifyDelegate("timestamp", previous);
        root.verifyDelegate("snapshot", stored);
        root.verifyDelegate("targets", authorization);
        if (currentBytes.equals(timestamp) && snapshotBytes.equals(snapshot)) {
            return;
        }
        const renewal = new Renewal(snapshot, timestamp, targetBytes, previous, root);

        // create an immutable snapshot without allowing conflicting content at the same revision
        const snapshotKey = `${this.prefix}metadata/${renewal.revision}.snapshot.json`;
        const created = await this.bucket.put(snapshotKey, renewal.snapshot, {
            onlyIf: { etagDoesNotMatch: "*" },
            httpMetadata: {
                contentType: "application/json",
                cacheControl: "public, max-age=31536000, immutable",
            },
        });
        if (!created) {
            const existing = await this.read(`metadata/${renewal.revision}.snapshot.json`);
            if (!Buffer.from(await existing.arrayBuffer()).equals(renewal.snapshot)) {
                throw new Error("snapshot revision already contains different content");
            }
        }

        // select the new snapshot only if no release or renewal replaced the timestamp concurrently
        const published = await this.bucket.put(
            `${this.prefix}metadata/timestamp.json`,
            renewal.timestamp,
            {
                onlyIf: { etagMatches: current.etag },
                httpMetadata: { contentType: "application/json", cacheControl: "no-store" },
            },
        );
        if (!published) {
            throw new Error(
                "repository changed during renewal; read the current revision and retry",
            );
        }
    }

    /** Load a bounded metadata document from authoritative storage. */
    private async read(path: string): Promise<R2ObjectBody> {
        const object = await this.bucket.get(`${this.prefix}${path}`);
        if (!object || object.size > METADATA_SIZE) {
            throw new Error(`missing or oversized release metadata: ${path}`);
        }

        return object;
    }

    /** Follow consecutive roots authenticated by both their predecessor and replacement quorum. */
    private async rotate(): Promise<Metadata<Root>> {
        // retain the configured root unless storage contains a consecutive authorized replacement
        let root = this.root;
        root.verifyDelegate("root", root);
        for (;;) {
            const object = await this.bucket.get(
                `${this.prefix}metadata/${root.signed.version + 1}.root.json`,
            );
            if (!object) {
                return root;
            }
            if (object.size > METADATA_SIZE) {
                throw new Error("oversized release root");
            }
            const next = Metadata.fromJSON(MetadataKind.Root, JSON.parse(await object.text()));
            if (next.signed.version !== root.signed.version + 1) {
                throw new Error("release root version must increase consecutively");
            }
            root.verifyDelegate("root", next);
            next.verifyDelegate("root", next);
            root = next;
        }
    }
}
