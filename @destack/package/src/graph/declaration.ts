import { defineSchema, Digest, schema } from "@destack/schema";
import { DeclarationName, PackageId } from "../definition/package.ts";
import { Moniker } from "./moniker.ts";

/** A Destack entity a module declares, such as a service, an object type or one of its methods. */
export const Declaration = Object.assign(
    defineSchema(
        schema.object({
            /** The declaration's moniker: its symbol's moniker, a member it derives, and its kind. */
            moniker: Moniker,
            /** The symbol declaring it. */
            symbol: Moniker,
            /** The declaration kind, such as `service`, `object` or `procedure`. */
            kind: DeclarationName,
            /** The package declaring the kind. */
            package: PackageId,
            /** The declaration's name within its kind. */
            name: schema.string().min(1),
            /** The description, parsed by the kind's schema at read. */
            description: schema.record(schema.string(), schema.json()),
            /** The declaration's terms, such as `object/note/relation/editor`, with the digest of each definition. */
            vocabulary: schema.record(schema.string(), Digest).exactOptional(),
        }),
    ),
    { isMember },
);
/** A Destack entity a module declares. */
export type Declaration = schema.Infer<typeof Declaration>;

/** Report whether a declaration is a member another declaration derives, such as a service's procedure. */
function isMember(declaration: Pick<Declaration, "moniker" | "symbol" | "kind">): boolean {
    return declaration.moniker !== `${declaration.symbol}:${declaration.kind}`;
}
