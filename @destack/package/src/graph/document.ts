import { Digest, schema } from "@destack/schema";
import { PackagePath } from "../file/file.ts";
import { CODE_EDGE_KINDS, Edge } from "./edge.ts";
import { Symbol } from "./symbol.ts";

/** One source file of the code index, after SCIP's `Document`: its symbols and their edges. */
export const Document = schema.object({
    /** The package-relative module path. */
    path: PackagePath,
    /** The digest of the source bytes the code was read from. */
    digest: Digest,
    /** The symbols and their members, by moniker. */
    symbols: schema.array(Symbol),
    /** The edges of the module's code: references, implementations, calls and coverage. */
    edges: schema.array(Edge.extend({ kind: schema.enum(CODE_EDGE_KINDS) })),
});
/** One source file of the code index. */
export type Document = schema.Infer<typeof Document>;
