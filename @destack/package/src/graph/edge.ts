import { defineSchema, schema } from "@destack/schema";
import { Moniker } from "./moniker.ts";

/** The kinds of edges. */
export const EDGE_KINDS = [
    // code
    "depends",
    "calls",
    "references",
    "implements",

    // declarations
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

/** A directed relation from a module, symbol or declaration to another. */
export const Edge = defineSchema(
    schema.object({
        /** The source moniker, in the module holding the edge. */
        from: Moniker,
        /** The target moniker, in any build. */
        to: Moniker,
        /** The relation. */
        kind: schema.enum(EDGE_KINDS),
    }),
);
/** A directed relation between monikers. */
export type Edge = schema.Infer<typeof Edge>;
