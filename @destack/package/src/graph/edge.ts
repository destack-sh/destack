import { schema } from "@destack/schema";
import { Moniker } from "./moniker.ts";

/** The kinds of a build's module edges: imports and every declaration edge. */
export const MODULE_EDGE_KINDS = [
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
] as const;

/** The kinds of the code index's edges: references, implementations, calls and test coverage. */
export const CODE_EDGE_KINDS = ["calls", "references", "implements", "covers"] as const;

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
