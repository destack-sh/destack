import { Digest, schema } from "@destack/schema";
import { SourceLocation } from "../source/location.ts";
import { DependencySymbol } from "../code/reference.ts";
import { DeclarationName, Package } from "./package.ts";
import { MemberSymbol } from "../graph/member.ts";

/** Where a stamped declaration constructor was called, as the module transform records it. */
export const DeclarationSite = schema.object({
    /** The package exporting the constructor. */
    package: schema.string().min(1),
    /** The constructor's export path, such as `defineObject`. */
    constructor: schema.string().min(1),
    /** The calling module, relative to its package. */
    module: schema.string().min(1),
    /** The call's zero-based line. */
    line: schema.number().int().min(0),
    /** The call's zero-based UTF-16 column. */
    column: schema.number().int().min(0),
    /** The module-level constant the call initializes: exported, unexported, or none. */
    binding: schema.enum(["exported", "unexported", "none"]),
});
/** Where a stamped declaration constructor was called. */
export type DeclarationSite = schema.Infer<typeof DeclarationSite>;

/** A declaration described by its domain inspector. */
export const DeclarationDescription = schema.object({
    /** The declaration name assigned by the domain inspector. */
    name: schema.string().min(1),
    /** The description kind. */
    kind: DeclarationName,
    /** The package declaring the kind. */
    package: Package,
    /** The original declaration in its exact source package. */
    symbol: DependencySymbol.extend({ package: Package }),
    /** The declaring module and source position. */
    source: SourceLocation,
    /** The description validated by its domain inspector. */
    description: schema.jsonObject(),
    /** The declaration's terms, such as `object/note/relation/editor`, with digests. */
    vocabulary: schema.record(schema.string(), Digest).exactOptional(),
    /** The symbols its kind derives with their relationships, if any. */
    symbols: schema.array(MemberSymbol).exactOptional(),
});
/** A declaration described by its domain inspector. */
export type DeclarationDescription = schema.Infer<typeof DeclarationDescription>;
