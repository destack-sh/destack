import { check, type Column, dialectSQL, integer, json, sql, uniqueIndex } from "@destack/db";
import { defineSchema, identifier, schema } from "@destack/schema";
import { PackageRelease } from "@destack/package/package";

/** The declaration and immutable revision last applied to a record. */
export const RecordProvenance = defineSchema(
    schema.union([
        schema.object({
            /** A declaration in an evaluated account definition. */
            kind: schema.literal("account"),
            /** The account following the declaration. */
            accountId: identifier("account"),
            /** The declaration name within its collection. */
            name: schema.string().min(1),
            /** The evaluated revision that last changed the record. */
            revisionId: identifier("account-revision"),
        }),
        schema.object({
            /** A declaration in an evaluated space definition. */
            kind: schema.literal("stack"),
            /** The space following the declaration. */
            spaceId: identifier("space"),
            /** The declaration name within its collection. */
            name: schema.string().min(1),
            /** The evaluated revision that last changed the record. */
            revisionId: identifier("space-revision"),
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

/** The declaration last applied to a persistent record. */
export type RecordProvenance = schema.Infer<typeof RecordProvenance>;

/** Add provenance to a table whose records can be declared in code. */
export function provenanceColumns() {
    return {
        /** The last applied declaration, absent for independently created records. */
        provenance: json("provenance", RecordProvenance),
        /** The time automatic application ended; null keeps a declared record source-managed. */
        detachedAt: integer("detached_at"),
    };
}

/** Constrain provenance and reserve each active declaration within this table. */
export function provenanceChecks(
    name: string,
    columns: {
        /** The last applied declaration. */
        provenance: Column;
        /** The time automatic application ended. */
        detachedAt: Column;
        /** The account containing this record, when account-scoped. */
        accountId?: Column;
        /** The space containing this record, when space-scoped. */
        spaceId?: Column;
    },
) {
    // constrain provenance ownership where the containing table records it
    const ownership = [];
    if (columns.accountId) {
        ownership.push(
            check(
                `${name}_provenance_account`,
                dialectSQL({
                    sqlite: sql`${columns.provenance} IS NULL OR json_extract(${columns.provenance}, '$.kind') <> 'account' OR json_extract(${columns.provenance}, '$.accountId') = ${columns.accountId}`,
                    postgresql: sql`${columns.provenance} IS NULL OR (${columns.provenance}::jsonb ->> 'kind') <> 'account' OR (${columns.provenance}::jsonb ->> 'accountId') = ${columns.accountId}`,
                }),
            ),
        );
    }

    // retain the declaring space when the record belongs to a space
    if (columns.spaceId) {
        ownership.push(
            check(
                `${name}_provenance_space`,
                dialectSQL({
                    sqlite: sql`${columns.provenance} IS NULL OR json_extract(${columns.provenance}, '$.kind') <> 'stack' OR json_extract(${columns.provenance}, '$.spaceId') = ${columns.spaceId}`,
                    postgresql: sql`${columns.provenance} IS NULL OR (${columns.provenance}::jsonb ->> 'kind') <> 'stack' OR (${columns.provenance}::jsonb ->> 'spaceId') = ${columns.spaceId}`,
                }),
            ),
        );
    }

    return [
        ...ownership,
        check(
            `${name}_provenance_detached`,
            sql`${columns.detachedAt} IS NULL OR (${columns.provenance} IS NOT NULL AND ${columns.detachedAt} >= 0)`,
        ),
        uniqueIndex(`${name}_provenance`)
            .on(
                dialectSQL({
                    sqlite: sql`json_extract(${columns.provenance}, '$.kind')`,
                    postgresql: sql`(${columns.provenance}::jsonb ->> 'kind')`,
                }),
                dialectSQL({
                    sqlite: sql`coalesce(json_extract(${columns.provenance}, '$.accountId'), json_extract(${columns.provenance}, '$.spaceId'), json_extract(${columns.provenance}, '$.installationId'))`,
                    postgresql: sql`coalesce((${columns.provenance}::jsonb ->> 'accountId'), (${columns.provenance}::jsonb ->> 'spaceId'), (${columns.provenance}::jsonb ->> 'installationId'))`,
                }),
                dialectSQL({
                    sqlite: sql`json_extract(${columns.provenance}, '$.name')`,
                    postgresql: sql`(${columns.provenance}::jsonb ->> 'name')`,
                }),
            )
            .where(sql`${columns.provenance} IS NOT NULL AND ${columns.detachedAt} IS NULL`),
    ];
}
