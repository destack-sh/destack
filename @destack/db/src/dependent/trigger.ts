import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { condition, literal, quote } from "../dialect/quote.ts";
import type { DependentDescription } from "../inspect/dependent.ts";
import { LOG_COPYING } from "../log/schema.ts";

/** The message refusing a deletion, classified as a broken reference. */
const RESTRICTED = "FOREIGN KEY constraint failed";

/** Generate the trigger cascading or refusing a row's deletion for its dependents. */
function install(dependent: DependentDescription, dialect: Dialect): string[] {
    // name the trigger and skip replica copies
    const name = quote(`${dependent.table}__${dependent.source}_${dependent.key}`);
    const idle = `NOT EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)})`;
    const source = quote(dependent.source);
    const rows = `${source} WHERE ${[
        `${source}.${quote(dependent.key)} = OLD.${quote(dependent.id)}`,
        ...dependent.where.map(
            (entry) => `${source}.${quote(entry.column)} ${condition(entry.value, dialect)}`,
        ),
    ].join(" AND ")}`;
    const message = `${RESTRICTED}: ${dependent.source} references ${dependent.table}`;

    // delete or refuse in a SQLite trigger
    if (dialect === "sqlite") {
        return [
            dependent.onDelete === "cascade"
                ? `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                    WHEN ${idle} BEGIN DELETE FROM ${rows}; END`
                : `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                    WHEN ${idle} AND EXISTS (SELECT 1 FROM ${rows})
                    BEGIN SELECT RAISE(ABORT, ${literal(message)}); END`,
        ];
    }
    // delete or refuse through one PostgreSQL function
    else if (dialect === "postgresql") {
        const effect =
            dependent.onDelete === "cascade"
                ? `DELETE FROM ${rows};`
                : `IF EXISTS (SELECT 1 FROM ${rows}) THEN
                      RAISE EXCEPTION USING ERRCODE = 'foreign_key_violation', MESSAGE = ${literal(message)};
                  END IF;`;

        return [
            `CREATE OR REPLACE FUNCTION ${name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)}) THEN RETURN NULL; END IF;
                ${effect}
                RETURN NULL;
            END $$`,
            `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                FOR EACH ROW EXECUTE FUNCTION ${name}()`,
        ];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Remove a dependent's trigger. */
function remove(dependent: DependentDescription, dialect: Dialect): string[] {
    const name = quote(`${dependent.table}__${dependent.source}_${dependent.key}`);

    // drop the SQLite trigger
    if (dialect === "sqlite") {
        return [`DROP TRIGGER IF EXISTS ${name}`];
    }
    // drop the PostgreSQL function with its trigger
    else if (dialect === "postgresql") {
        return [`DROP FUNCTION IF EXISTS ${name}() CASCADE`];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** The triggers keeping dependents consistent. */
export const dependentTriggers: Triggers = {
    install: (state, dialect) =>
        (state.dependents ?? []).flatMap((dependent) => install(dependent, dialect)),
    remove: (state, dialect) =>
        (state.dependents ?? []).flatMap((dependent) => remove(dependent, dialect)),
};
