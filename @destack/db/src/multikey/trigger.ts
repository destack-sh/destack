import type { TriggerSet } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";
import type { MultikeyIndexDescription } from "./description.ts";

/** Generate the triggers writing a multikey index's entries with every write to its source rows. */
function install(multikey: MultikeyIndexDescription, dialect: Dialect): string[] {
    // derive the names and the statements per row
    const prefix = quote(`${multikey.entryTable}_write`);
    const source = quote(multikey.table);
    const drop = (row: string) =>
        `DELETE FROM ${quote(multikey.entryTable)} WHERE ${quote("row")} = ${row}.${quote(multikey.id)};`;

    // write them with SQLite triggers on every database
    if (dialect === "sqlite") {
        const add = `${insert(multikey, dialect, "NEW")};`;
        const name = (suffix: string) => quote(`${multikey.entryTable}_write_${suffix}`);

        return [
            `CREATE TRIGGER ${name("insert")} AFTER INSERT ON ${source} BEGIN ${drop("NEW")} ${add} END`,
            `CREATE TRIGGER ${name("update")} AFTER UPDATE ON ${source} BEGIN ${drop("OLD")} ${drop("NEW")} ${add} END`,
            `CREATE TRIGGER ${name("delete")} AFTER DELETE ON ${source} BEGIN ${drop("OLD")} END`,
        ];
    }
    // write them through one PostgreSQL function
    else if (dialect === "postgresql") {
        return [
            `CREATE OR REPLACE FUNCTION ${prefix}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF TG_OP <> 'INSERT' THEN ${drop("OLD")} END IF;
                IF TG_OP <> 'DELETE' THEN ${drop("NEW")} ${insert(multikey, dialect, "NEW")}; END IF;
                RETURN NULL;
            END $$`,
            `CREATE TRIGGER ${prefix} AFTER INSERT OR UPDATE OR DELETE ON ${source}
                FOR EACH ROW EXECUTE FUNCTION ${prefix}()`,
        ];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Remove an index's triggers. */
function remove(multikey: MultikeyIndexDescription, dialect: Dialect): string[] {
    // drop each SQLite trigger
    if (dialect === "sqlite") {
        return ["insert", "update", "delete"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${multikey.entryTable}_write_${suffix}`)}`,
        );
    }
    // drop the PostgreSQL function with its trigger
    else if (dialect === "postgresql") {
        return [`DROP FUNCTION IF EXISTS ${quote(`${multikey.entryTable}_write`)}() CASCADE`];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Recompute every entry of a multikey index from its source rows. */
export function rebuildMultikeyIndex(
    multikey: MultikeyIndexDescription,
    dialect: Dialect,
): string[] {
    return [`DELETE FROM ${quote(multikey.entryTable)}`, insert(multikey, dialect, undefined)];
}

/** Select the entries of one source row, or of every row, into the entry table. */
function insert(
    multikey: MultikeyIndexDescription,
    dialect: Dialect,
    row: string | undefined,
): string {
    // read each element with its place, from one row or every row
    const columns = ["row", "position", ...multikey.fields.map((field) => field.column)];
    const owner = row ?? "source";
    const from = `${owner}.${quote(multikey.from)}`;
    const id = `${owner}.${quote(multikey.id)}`;
    const elements =
        dialect === "sqlite"
            ? `json_each(${from}) AS element`
            : `jsonb_array_elements(${from}::jsonb) WITH ORDINALITY AS element(value, ordinal)`;
    const position = dialect === "sqlite" ? "element.key" : "element.ordinal - 1";
    const values = multikey.fields.map((field) => read(field, dialect, owner));
    const elementValues = multikey.fields.flatMap((field) =>
        "source" in field ? [] : [`${read(field, dialect, owner)} IS NOT NULL`],
    );
    const sparse = multikey.isSparse ? ` WHERE ${elementValues.join(" OR ")}` : "";
    const sources =
        row === undefined ? `${quote(multikey.table)} AS source, ${elements}` : elements;

    return `INSERT INTO ${quote(multikey.entryTable)} (${columns.map((column) => quote(column)).join(", ")})
        SELECT ${[id, position, ...values].join(", ")} FROM ${sources}${sparse}`;
}

/** Read one field of an element, or of its source row, as its kind stores it. */
function read(
    field: MultikeyIndexDescription["fields"][number],
    dialect: Dialect,
    owner: string,
): string {
    // copy a column of the source row
    if ("source" in field) {
        return `${owner}.${quote(field.source)}`;
    }
    // read through SQLite's JSON path operators
    else if (dialect === "sqlite") {
        const path = literal(`$.${field.path.join(".")}`);

        return field.kind === "json" ? `element.value -> ${path}` : `element.value ->> ${path}`;
    }

    // read through PostgreSQL's path operators, casting integers
    const path = literal(`{${field.path.join(",")}}`);
    if (field.kind === "json") {
        return `element.value #> ${path}`;
    }

    return field.kind === "integer"
        ? `(element.value #>> ${path})::integer`
        : `element.value #>> ${path}`;
}

/** The triggers of multikey indexes. */
export const multikeyIndexTriggers: TriggerSet = {
    install: (state, dialect) =>
        (state.multikeyIndexes ?? []).flatMap((each) => install(each, dialect)),
    remove: (state, dialect) =>
        (state.multikeyIndexes ?? []).flatMap((each) => remove(each, dialect)),
    tables: (state) =>
        (state.multikeyIndexes ?? []).flatMap((each) => [each.table, each.entryTable]),
};
