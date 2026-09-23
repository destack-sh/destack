import { check, type Column, sql } from "@destack/db";

/** Constrain the generation and export selected by an account or space. */
export function sourceChecks(
    name: string,
    columns: {
        /** The selected generation. */
        generation: Column;
        /** The relative package directory. */
        directory: Column;
        /** The exported definition. */
        export: Column;
        /** The public entrypoint. */
        entrypoint: Column;
    },
) {
    return [
        check(`${name}_generation`, sql`${columns.generation} > 0`),
        check(
            `${name}_directory`,
            sql`length(${columns.directory}) > 0 AND substr(${columns.directory}, 1, 1) <> '/' AND ${columns.directory} <> '..' AND ${columns.directory} NOT LIKE '../%' AND ${columns.directory} NOT LIKE '%/../%' AND ${columns.directory} NOT LIKE '%/..'`,
        ),
        check(
            `${name}_export`,
            sql`length(${columns.export}) > 0 AND (${columns.entrypoint} = '.' OR ${columns.entrypoint} LIKE './_%')`,
        ),
    ];
}
