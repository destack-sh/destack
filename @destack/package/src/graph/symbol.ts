import { schema } from "@destack/schema";
import { SourceRange } from "../source/location.ts";
import { Moniker } from "./moniker.ts";

/** One declaration of a symbol: its kind as SCIP kinds them, its range and its signature. */
export const SymbolDeclaration = schema.object({
    /** The kind of entity it declares. */
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
    /** Its source range. */
    source: SourceRange,
    /** Its signature as written, one line. */
    signature: schema.string(),
});
/** One declaration of a symbol. */
export type SymbolDeclaration = schema.Infer<typeof SymbolDeclaration>;

/** A code entity of a module: a declared name or a member, with each of its declarations. */
export const Symbol = schema.object({
    /** The symbol's moniker. */
    moniker: Moniker,
    /** Its declarations in source order: a value and a type, merged ones, or overloads. */
    declarations: schema.array(SymbolDeclaration).min(1),
    /** The first sentence of the first declaration's documentation, absent without one. */
    comment: schema.string().min(1).exactOptional(),
    /** Whether a package module exports the symbol, or the symbol of its member. */
    isExported: schema.boolean(),
});
/** A code entity of a module. */
export type Symbol = schema.Infer<typeof Symbol>;
