import { defineSchema, schema } from "@destack/schema";
import { SourceRange } from "../source/location.ts";
import { Moniker } from "./moniker.ts";

/** The kinds of code symbols. */
export const SYMBOL_KINDS = [
    "function",
    "class",
    "interface",
    "type",
    "variable",
    "enum",
    "namespace",
    "method",
    "property",
    "accessor",
    "member",
] as const;

/** A code entity of a module: a declared name or one of its members. */
export const Symbol = defineSchema(
    schema.object({
        /** The symbol's moniker. */
        moniker: Moniker,
        /** The kind of entity. */
        kind: schema.enum(SYMBOL_KINDS),
        /** The first declaration's source range. */
        source: SourceRange,
        /** The printed declaration, one line per overload. */
        signature: schema.string(),
        /** The first sentence of the documentation, absent without documentation. */
        comment: schema.string().min(1).exactOptional(),
        /** Whether a module of the package exports the symbol, or the symbol a member belongs to. */
        isExported: schema.boolean(),
    }),
);
/** A code entity of a module. */
export type Symbol = schema.Infer<typeof Symbol>;
