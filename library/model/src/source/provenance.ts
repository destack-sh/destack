import { check, type Column, dialectSQL, integer, json, sql, uniqueIndex } from "@destack/db";
import { RecordSource } from "./source.ts";

/** Add source provenance to a table whose records can be declared in code. */
export function provenanceColumns() {
    return {
        /** The last applied declaration, absent for independently created records. */
        source: json("source", RecordSource),
        /** The time source control ended; null keeps a declared record source-managed. */
        sourceDetachedAt: integer("source_detached_at"),
    };
}

/** Constrain provenance and reserve each active declaration within this table. */
export function provenanceChecks(name: string, columns: {
    /** The last applied source declaration. */
    source: Column;
    /** The time source control ended. */
    sourceDetachedAt: Column;
}) {
    return [
        check(
            `${name}_source_detached`,
            sql`${columns.sourceDetachedAt} IS NULL OR (${columns.source} IS NOT NULL AND ${columns.sourceDetachedAt} >= 0)`,
        ),
        uniqueIndex(`${name}_source`).on(
            dialectSQL({
                sqlite: sql`json_extract(${columns.source}, '$.kind')`,
                postgresql: sql`(${columns.source}::jsonb ->> 'kind')`,
            }),
            dialectSQL({
                sqlite:
                    sql`coalesce(json_extract(${columns.source}, '$.spaceId'), json_extract(${columns.source}, '$.installationId'))`,
                postgresql:
                    sql`coalesce((${columns.source}::jsonb ->> 'spaceId'), (${columns.source}::jsonb ->> 'installationId'))`,
            }),
            dialectSQL({
                sqlite: sql`json_extract(${columns.source}, '$.name')`,
                postgresql: sql`(${columns.source}::jsonb ->> 'name')`,
            }),
        ).where(sql`${columns.source} IS NOT NULL AND ${columns.sourceDetachedAt} IS NULL`),
    ];
}
