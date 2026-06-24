import type { LocalNodeId } from "../../_generated/mir/tree/node.js";

/** Return whether two local node ids refer to the same MIR node. */
export function localNodeIdMatches(left: LocalNodeId, right: LocalNodeId): boolean {
    return left.id === right.id;
}

/** Return one map value keyed by local node id. */
export function getLocalNodeValue<Value>(
    map: ReadonlyMap<LocalNodeId, Value>,
    key: LocalNodeId,
): Value | undefined {
    for (const [candidate, value] of map) {
        // compare structural node ids because JS maps key objects by identity
        if (localNodeIdMatches(candidate, key)) {
            return value;
        }
    }

    return undefined;
}
