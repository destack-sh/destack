import { defineSchema, schema } from "@destack/schema";

/** A scoped Destack package name. */
export const PackageName = defineSchema(
    schema.string().max(214).regex(
        /^@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*$(?![\s\S])/,
    ),
);

/** A scoped or unscoped dependency name. */
export const DependencyName = defineSchema(
    schema.string().max(214).regex(
        /^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$(?![\s\S])/,
    ),
);

/** The name and version declared by a package. */
export const Package = defineSchema(schema.object({
    /** The package name, qualified by its owner. */
    name: PackageName,
    /** The package version. */
    version: schema.string().min(1),
}));

/** A named, versioned package. */
export type Package = Readonly<schema.Infer<typeof Package>>;
