import type { graph } from "@destack/package";
import { stringifyInspection } from "./serialization.ts";
import type { JsonValue } from "@destack/schema";

/** The lists a manifest refers to, each kept in `manifest/<list>.json`. */
export const MANIFEST_LISTS = ["dependencies", "files", "sourceMaps", "graph"] as const;

/** Shared descriptions collected across build targets. */
export interface ManifestDescription {
    /** The graph file of each module. */
    graph: graph.Module[];
}

/** Encode descriptions as readable JSON without implicit value conversions. */
export function encodeDescription(value: JsonValue): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${stringifyInspection(value, 4)}\n`);
}
