import { Key, Metadata, MetadataKind, Root } from "@tufjs/models";
import { readFile } from "node:fs/promises";

/** Independent public keys for automated metadata roles. */
export interface ReleasePublicKey {
    /** Key authorizing release artifacts. */
    targets: Key;
    /** Key authorizing a consistent metadata snapshot. */
    snapshot: Key;
    /** Key authorizing snapshot freshness. */
    timestamp: Key;
}

/** Create unsigned two-of-three root metadata using only public keys. */
export function createRoot(
    version: number,
    roots: Key[],
    keys: ReleasePublicKey,
    expires: string,
): Metadata<Root> {
    // require independent keys for the root quorum and each online role
    const all = [...roots, keys.targets, keys.snapshot, keys.timestamp];
    if (
        roots.length !== 3 ||
        new Set(all.map((key) => key.keyID)).size !== 6 ||
        new Set(all.map((key) => key.keyVal.public)).size !== 6
    ) {
        throw new Error("root requires three independent keys and three distinct online keys");
    }
    if (
        !Number.isSafeInteger(version) ||
        version < 1 ||
        !Number.isFinite(Date.parse(expires)) ||
        Date.parse(expires) <= Date.now()
    ) {
        throw new Error("root requires a positive version and future expiration");
    }

    // describe authorization without loading any private root material
    const root = Root.fromJSON({
        _type: "root",
        version,
        spec_version: "1.0.31",
        expires,
        consistent_snapshot: true,
        keys: {},
        roles: {
            root: { keyids: [], threshold: 2 },
            targets: { keyids: [], threshold: 1 },
            snapshot: { keyids: [], threshold: 1 },
            timestamp: { keyids: [], threshold: 1 },
        },
    });
    for (const key of roots) {
        root.addKey(key, "root");
    }
    for (const role of ["targets", "snapshot", "timestamp"] as const) {
        root.addKey(keys[role], role);
    }

    return new Metadata(root);
}

/** Verify initial trust or a consecutive root rotation against both required quorums. */
export function verifyRoot(root: Metadata<Root>, previous?: Metadata<Root>): void {
    // require current authorization for completed signing ceremonies
    if (root.signed.isExpired()) {
        throw new Error("root has expired");
    }
    authenticateRoot(root, previous);
}

/** Authenticate bootstrap trust or a consecutive rotation, including expired historical roots. */
export function authenticateRoot(root: Metadata<Root>, previous?: Metadata<Root>): void {
    // preserve the agreed root quorum independently of signature validity
    const role = root.signed.roles.root;
    if (role?.threshold !== 2 || role.keyIDs.length !== 3 || new Set(role.keyIDs).size !== 3) {
        throw new Error("root policy must require two of three independent keys");
    }

    // reject accidental gaps in the rotation history
    const version = previous ? previous.signed.version + 1 : 1;
    if (root.signed.version !== version) {
        throw new Error(`expected root version ${version}`);
    }

    // require authorization from both the previous and replacement root key sets
    root.verifyDelegate("root", root);
    if (previous) {
        previous.verifyDelegate("root", previous);
        previous.verifyDelegate("root", root);
    }
}

/** Read a public root document, including partially signed ceremony documents. */
export async function readRoot(path: string): Promise<Metadata<Root>> {
    return Metadata.fromJSON(MetadataKind.Root, JSON.parse(await readFile(path, "utf8")));
}
