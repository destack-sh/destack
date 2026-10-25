import { Key, Metadata, MetadataKind, Root } from "@tufjs/models";
import type { Environment } from "@destack/environment";
import { parseObject } from "./json.ts";
import { UpdateError } from "../error/index.ts";

/** Independent public keys for automated metadata roles. */
export interface ReleasePublicKey {
    /** Key authorizing release artifacts. */
    targets: Key;
    /** Key authorizing a consistent metadata snapshot. */
    snapshot: Key;
    /** Key authorizing snapshot freshness. */
    timestamp: Key;
}

/** The root keys of the two-of-three quorum. */
const ROOT_KEYS = 3;

/** The online keys: targets, snapshot and timestamp. */
const ONLINE_KEYS = 3;

/** The signed root metadata clients pin, as Sigstore's trusted root. */
export const TrustedRoot = {
    create: createRoot,
    verify: verifyRoot,
    authenticate: authenticateRoot,
    read: readRoot,
};

/** Create unsigned two-of-three root metadata using only public keys. */
function createRoot(
    version: number,
    roots: Key[],
    keys: ReleasePublicKey,
    expires: string,
    environment: Environment,
): Metadata<Root> {
    // require independent keys for the root quorum and each online role
    const all = [...roots, keys.targets, keys.snapshot, keys.timestamp];
    const keyCount = ROOT_KEYS + ONLINE_KEYS;
    if (
        roots.length !== ROOT_KEYS ||
        new Set(all.map((key) => key.keyID)).size !== keyCount ||
        new Set(all.map((key) => key.keyVal["public"])).size !== keyCount
    ) {
        throw UpdateError.of(
            "METADATA_INVALID",
            "root requires three independent keys and three distinct online keys",
        );
    }
    if (
        !Number.isSafeInteger(version) ||
        version < 1 ||
        !Number.isFinite(Date.parse(expires)) ||
        Date.parse(expires) <= environment.clock.now()
    ) {
        throw UpdateError.of(
            "METADATA_INVALID",
            "root requires a positive version and future expiration",
        );
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
function verifyRoot(root: Metadata<Root>, previous?: Metadata<Root>): void {
    // require current authorization for completed signing ceremonies
    if (root.signed.isExpired()) {
        throw UpdateError.of("METADATA_INVALID", "root has expired");
    }
    authenticateRoot(root, previous);
}

/** Authenticate bootstrap trust or a consecutive rotation, including expired historical roots. */
function authenticateRoot(root: Metadata<Root>, previous?: Metadata<Root>): void {
    // preserve the agreed root quorum independently of signature validity
    const role = root.signed.roles["root"];
    if (role?.threshold !== 2 || role.keyIDs.length !== 3 || new Set(role.keyIDs).size !== 3) {
        throw UpdateError.of(
            "METADATA_INVALID",
            "root policy must require two of three independent keys",
        );
    }

    // reject accidental gaps in the rotation history
    const version = previous ? previous.signed.version + 1 : 1;
    if (root.signed.version !== version) {
        throw UpdateError.of("METADATA_INVALID", `expected root version ${version}`);
    }

    // require authorization from both the previous and replacement root key sets
    root.verifyDelegate("root", root);
    if (previous) {
        previous.verifyDelegate("root", previous);
        previous.verifyDelegate("root", root);
    }
}

/** Read a public root document, including partially signed ceremony documents. */
async function readRoot(path: string, environment: Environment<"files">): Promise<Metadata<Root>> {
    const text = await environment.files.readTextFile(path);

    return Metadata.fromJSON(MetadataKind.Root, parseObject(text));
}
