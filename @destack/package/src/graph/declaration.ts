import { Digest, schema } from "@destack/schema";
import { DeclarationName, PackageId } from "../definition/package.ts";
import { Moniker } from "./moniker.ts";

/** A Destack entity a module declares, such as a service, an object type or one of its methods. */
export const Declaration = Object.assign(
    schema.object({
        /** The declaration's moniker: its symbol's with a member and kind, or a kind and name. */
        moniker: Moniker,
        /** The symbol declaring it. */
        symbol: Moniker,
        /** The declaration kind, such as `service`, `object` or `procedure`. */
        kind: DeclarationName,
        /** The package defining the declaration's kind. */
        kindPackageId: PackageId,
        /** The declaration's name within its kind. */
        name: schema.string().min(1),
        /** The description, parsed by the kind's schema at read. */
        description: schema.jsonObject(),
        /** The declaration's terms, such as `object/note/relation/editor`, with digests. */
        vocabulary: schema.record(schema.string(), Digest).exactOptional(),
    }),
    { at, member, isMember },
);
/** A Destack entity a module declares. */
export type Declaration = schema.Infer<typeof Declaration>;

/** Declare a symbol as a kind: the moniker is the symbol's with the kind and the name. */
function at(symbol: Moniker, declared: Omit<Declaration, "moniker" | "symbol">): Declaration {
    return { moniker: monikerOf(symbol, declared), symbol, ...declared };
}

/** Declare a member another declaration derives, such as an object's procedure, at its symbol. */
function member(
    parent: Pick<Declaration, "symbol" | "kindPackageId">,
    derived: Pick<Declaration, "kind" | "name" | "description">,
): Declaration {
    const moniker = Moniker.parse(`${parent.symbol}.${derived.name}:${derived.kind}`);

    return { moniker, symbol: parent.symbol, kindPackageId: parent.kindPackageId, ...derived };
}

/** Report whether a declaration is a member another one derives, such as a procedure. */
function isMember(declaration: Pick<Declaration, "moniker" | "symbol" | "kind" | "name">): boolean {
    return declaration.moniker !== monikerOf(declaration.symbol, declaration);
}

/** Name a declaration at its symbol by its kind and name. */
function monikerOf(symbol: Moniker, declared: Pick<Declaration, "kind" | "name">): Moniker {
    return Moniker.parse(`${symbol}:${declared.kind}:${declared.name}`);
}
