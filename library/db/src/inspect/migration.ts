import { defineSchema, schema } from "@destack/schema";

/** A committed database migration. */
export const MigrationDescription = defineSchema(
    schema.object({
        /** The committed migration directory name. */
        name: schema.string().min(1),
        /** The SHA-256 digest of the SQL file. */
        checksum: schema.string().regex(/^[a-f0-9]{64}$/),
    }),
);
/** A committed database migration. */
export type MigrationDescription = schema.Infer<typeof MigrationDescription>;

/** A migration applied to a live database. */
export const MigrationRecord = defineSchema(
    MigrationDescription.extend({
        /** The UTC time recorded when applying the migration. */
        appliedAt: schema.iso.datetime(),
    }),
);
/** A migration applied to a live database. */
export type MigrationRecord = schema.Infer<typeof MigrationRecord>;
