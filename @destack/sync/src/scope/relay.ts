import type { LogPosition } from "@destack/db/log";
import type { QueryPage } from "../query/index.ts";
import type { ObjectTypeReference } from "./reference.ts";

/** Streams the scopes of a chain to a database below them. */
export interface ChainRelay {
    /** The scope at the bottom of the chain. */
    readonly scope: string;
    /** Stream one scope's copy for a database's held and copied types, from a position or a snapshot. */
    watch(
        request: {
            readonly scope: string;
            readonly held: readonly ObjectTypeReference[];
            readonly copied: readonly ObjectTypeReference[];
            readonly after?: LogPosition;
        },
        signal: AbortSignal,
    ): AsyncIterable<QueryPage>;
}
