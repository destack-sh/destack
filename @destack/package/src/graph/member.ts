import { schema } from "@destack/schema";
import { DeclarationName, PackageId } from "../definition/package.ts";
import { EDGE_KINDS } from "./edge.ts";

/** The declarations a relationship ends at, named by kind and name, resolved in the build. */
export const Target = schema.object({
    /** The declaration kind. */
    kind: DeclarationName,
    /** The name within the kind. */
    name: schema.string().min(1),
    /** The package declaring the symbol, the referring declaration's package when absent. */
    packageId: PackageId.exactOptional(),
    /** The member's declaration by kind and name, any of the package's when absent. */
    parent: schema
        .object({
            /** The declaration kind. */
            kind: DeclarationName,
            /** The name within the kind. */
            name: schema.string().min(1),
        })
        .exactOptional(),
});
/** The declarations a relationship ends at, named by kind and name. */
export type Target = schema.Infer<typeof Target>;

/** A relation from a symbol to the declarations of a target, after SCIP's `Relationship`. */
export const Relationship = schema.object({
    /** The relation. */
    kind: schema.enum(EDGE_KINDS),
    /** The declarations the relationship ends at. */
    symbol: Target,
});
/** A relation from a symbol to the declarations of a target. */
export type Relationship = schema.Infer<typeof Relationship>;

/** A member a declaration has with its relationships, as its kind's `symbols` function lists it. */
export const MemberSymbol = schema.object({
    /** The member, such as a procedure with its description, the declaration if absent. */
    member: schema
        .object({
            /** The member's kind. */
            kind: DeclarationName,
            /** The member's name within its kind. */
            name: schema.string().min(1),
            /** The member's description. */
            description: schema.jsonObject(),
        })
        .exactOptional(),
    /** The relationships from the symbol to the declarations they name. */
    relationships: schema.array(Relationship),
});
/** A member a declaration has with its relationships. */
export type MemberSymbol = schema.Infer<typeof MemberSymbol>;
