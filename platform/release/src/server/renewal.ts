import { Metadata, MetadataKind, type Root, type Timestamp } from "@tufjs/models";

/** Maximum snapshot lifetime accepted by the publication service, in milliseconds. */
const SNAPSHOT_LIFETIME_MS = 7 * 86_400_000;
/** Maximum timestamp lifetime accepted by the publication service, in milliseconds. */
const TIMESTAMP_LIFETIME_MS = 2 * 86_400_000;

/** Authenticated freshness metadata that preserves the currently published targets. */
export class Renewal {
    /** Exact signed snapshot bytes to publish under the new revision. */
    readonly snapshot: Buffer;
    /** Exact signed timestamp bytes to publish after its snapshot. */
    readonly timestamp: Buffer;
    /** Monotonic snapshot revision used in the immutable object name. */
    readonly revision: number;

    /** Verify a renewal against trusted storage and the current root's role keys. */
    constructor(
        snapshot: Buffer,
        timestamp: Buffer,
        targets: Buffer,
        previous: Metadata<Timestamp>,
        root: Metadata<Root>,
        now = new Date(),
    ) {
        // authenticate both new roles and the exact existing targets authorization
        const nextSnapshot = Metadata.fromJSON(
            MetadataKind.Snapshot,
            JSON.parse(snapshot.toString()),
        );
        const nextTimestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            JSON.parse(timestamp.toString()),
        );
        const currentTargets = Metadata.fromJSON(
            MetadataKind.Targets,
            JSON.parse(targets.toString()),
        );
        root.verifyDelegate("timestamp", nextTimestamp);
        root.verifyDelegate("snapshot", nextSnapshot);
        root.verifyDelegate("targets", currentTargets);

        // require fresh authorization while allowing recovery of expired stored freshness metadata
        if (root.signed.isExpired(now) || currentTargets.signed.isExpired(now)) {
            throw new Error("root and targets must be renewed before freshness metadata");
        }
        for (const [metadata, lifetime] of [
            [nextSnapshot, SNAPSHOT_LIFETIME_MS],
            [nextTimestamp, TIMESTAMP_LIFETIME_MS],
        ] as const) {
            const expires = Date.parse(metadata.signed.expires);
            if (
                !Number.isFinite(expires) ||
                expires <= now.getTime() ||
                expires > now.getTime() + lifetime
            ) {
                throw new Error("renewal expiration exceeds its permitted lifetime");
            }
        }

        // reject rollback and preserve the only targets document selected by trusted storage
        if (
            nextTimestamp.signed.version <= previous.signed.version ||
            nextSnapshot.signed.version <= previous.signed.snapshotMeta.version
        ) {
            throw new Error("renewal revisions must increase");
        }
        const reference = nextSnapshot.signed.meta["targets.json"];
        if (
            Object.keys(nextSnapshot.signed.meta).length !== 1 ||
            !reference ||
            reference.version !== currentTargets.signed.version ||
            nextTimestamp.signed.snapshotMeta.version !== nextSnapshot.signed.version
        ) {
            throw new Error("renewal must preserve the current targets document");
        }

        // require exact SHA-256 references before making either document available for publication
        for (const [metadataReference, bytes] of [
            [reference, targets],
            [nextTimestamp.signed.snapshotMeta, snapshot],
        ] as const) {
            if (metadataReference.length !== bytes.length || !metadataReference.hashes?.sha256) {
                throw new Error("renewal requires exact metadata length and SHA-256");
            }
            metadataReference.verify(bytes);
        }
        this.snapshot = snapshot;
        this.timestamp = timestamp;
        this.revision = nextSnapshot.signed.version;
    }
}
