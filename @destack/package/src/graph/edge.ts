import { schema } from "@destack/schema";
import { Moniker } from "./moniker.ts";

/** The kinds of edges, of modules and of code. */
export const EDGE_KINDS = [
    // modules and declarations
    "depends",
    "serves",
    "writes",
    "reads",
    "invokes",
    "binds",
    "presents",
    "renders",
    "emits",
    "covers",
    "shows",

    // code
    "calls",
    "references",
    "implements",
] as const;

/** A directed relation from a module, symbol or declaration to another. */
export const Edge = schema.object({
    /** The source moniker, in the module with the edge. */
    from: Moniker,
    /** The target moniker, in any build. */
    to: Moniker,
    /** The relation. */
    kind: schema.enum(EDGE_KINDS),
});
/** A directed relation between monikers. */
export type Edge = schema.Infer<typeof Edge>;
