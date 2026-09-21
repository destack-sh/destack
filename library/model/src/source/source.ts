import { defineSchema, identifier, schema } from "@destack/schema";
import { PackageRelease } from "@destack/package/package";

/** The declaration and immutable revision last applied to a record. */
export const RecordSource = defineSchema(
    schema.union([
        schema.object({
            /** A declaration in an evaluated space configuration. */
            kind: schema.literal("stack"),
            /** The space selecting the configuration. */
            spaceId: identifier("space"),
            /** The declaration name within its collection. */
            name: schema.string().min(1),
            /** The evaluated configuration that last changed the record. */
            revisionId: identifier("stack-revision"),
        }),
        schema.object({
            /** A declaration supplied by an installed package. */
            kind: schema.literal("package"),
            /** The installation supplying the declaration. */
            installationId: identifier("installation"),
            /** The stable declaration name within its collection. */
            name: schema.string().min(1),
            /** The immutable package release that last changed the record. */
            release: PackageRelease,
        }),
    ]),
);

/** The source declaration last applied to a persistent record. */
export type RecordSource = schema.Infer<typeof RecordSource>;
