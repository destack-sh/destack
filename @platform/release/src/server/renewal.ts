import { Metadata, MetadataKind, type MetaFile, type Root, type Timestamp } from "@tufjs/models";
import { LIFETIME_DAYS } from "@destack/update/publish";
import { parseDocument } from "../repository/document.ts";

/** The milliseconds in one day. */
const DAY_MS = 86_400_000;

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
            parseDocument(snapshot.toString()),
        );
        const nextTimestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            parseDocument(timestamp.toString()),
        );
        const currentTargets = Metadata.fromJSON(
            MetadataKind.Targets,
            parseDocument(targets.toString()),
        );
        root.verifyDelegate("timestamp", nextTimestamp);
        root.verifyDelegate("snapshot", nextSnapshot);
        root.verifyDelegate("targets", currentTargets);

        // require fresh authorization while allowing recovery of expired stored freshness metadata
        if (root.signed.isExpired(now) || currentTargets.signed.isExpired(now)) {
            throw new Error("root and targets must be renewed before freshness metadata");
        }
        requireLifetime(nextSnapshot.signed.expires, LIFETIME_DAYS.snapshot * DAY_MS, now);
        requireLifetime(nextTimestamp.signed.expires, LIFETIME_DAYS.timestamp * DAY_MS, now);

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
        requireExactReference(reference, targets);
        requireExactReference(nextTimestamp.signed.snapshotMeta, snapshot);
        this.snapshot = snapshot;
        this.timestamp = timestamp;
        this.revision = nextSnapshot.signed.version;
    }
}

/** Require an expiry in the future and within a role's permitted lifetime. */
function requireLifetime(expires: string, lifetime: number, now: Date): void {
    const time = Date.parse(expires);
    if (!Number.isFinite(time) || time <= now.getTime() || time > now.getTime() + lifetime) {
        throw new Error("renewal expiration exceeds its permitted lifetime");
    }
}

/** Require a metadata reference to state the exact length and SHA-256 of a document. */
function requireExactReference(reference: MetaFile, bytes: Buffer): void {
    if (reference.length !== bytes.length || reference.hashes?.["sha256"] === undefined) {
        throw new Error("renewal requires exact metadata length and SHA-256");
    }
    reference.verify(bytes);
}
