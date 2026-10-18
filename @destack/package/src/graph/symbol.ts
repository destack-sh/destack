import { schema } from "@destack/schema";
import { SourceRange } from "../source/location.ts";
import { Moniker } from "./moniker.ts";

/** A code entity of a module: a declared name or one of its members, as SCIP kinds them. */
export const Symbol = schema.object({
    /** The symbol's moniker. */
    moniker: Moniker,
    /** The kind of entity. */
    kind: schema.enum([
        // callables and types
        "function",
        "class",
        "interface",
        "typeAlias",
        "typeParameter",
        "enum",
        "namespace",
        "module",

        // values
        "variable",
        "parameter",

        // members
        "method",
        "constructor",
        "property",
        "getter",
        "setter",
        "enumMember",
    ]),
    /** The first declaration's source range. */
    source: SourceRange,
    /** The printed declaration, one line per overload. */
    signature: schema.string(),
    /** The first sentence of the documentation, absent without documentation. */
    comment: schema.string().min(1).exactOptional(),
    /** Whether a package module exports the symbol, or the symbol of its member. */
    isExported: schema.boolean(),
});
/** A code entity of a module. */
export type Symbol = schema.Infer<typeof Symbol>;
