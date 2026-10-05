import { defineSchema, schema } from "@destack/schema";

/** The pattern of a moniker: a package and a module path followed by an optional symbol, member and declaration kind. */
const MONIKER_PATTERN =
    /^(?:package-[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}|npm:(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*)\/[^#\x00-\x1f\x7f]+(?:#[^\x00-\x1f\x7f]+)?$(?![\s\S])/u;

/** The schema of a moniker. */
const monikerSchema = defineSchema(schema.string().regex(MONIKER_PATTERN));

/**
 * A stable name of a module, symbol or declaration across builds, such as `package-…/src/note.ts#Note.update`.
 *
 * Destack packages are named by their identifier, npm packages and compiler libraries by `npm:<name>`.
 * A declaration appends its kind to the symbol it is declared at, such as `#notes:service`.
 */
export type Moniker = schema.Infer<typeof monikerSchema>;

/** A stable name of a module, symbol or declaration across builds. */
export const Moniker = Object.assign(monikerSchema, { of });

/** The names a moniker joins: a package and a module followed by an optional symbol, member and declaration kind. */
export interface MonikerName {
    /** The package identifier, or `npm:<name>` for a package without one. */
    readonly packageId: string;
    /** The package-relative module path. */
    readonly module: string;
    /** The qualified symbol name, absent for the module itself. */
    readonly name?: string;
    /** The member of the symbol. */
    readonly member?: string;
    /** The declaration kind, for a declaration at the symbol or member. */
    readonly kind?: string;
}

/** Join a moniker from its names. */
function of(name: MonikerName): Moniker {
    // name the module followed by the symbol, its member and the declaration kind
    const symbol = name.name === undefined ? "" : `#${name.name}`;
    const member = name.member === undefined ? "" : `.${name.member}`;
    const kind = name.kind === undefined ? "" : `:${name.kind}`;

    return monikerSchema.parse(`${name.packageId}/${name.module}${symbol}${member}${kind}`);
}
