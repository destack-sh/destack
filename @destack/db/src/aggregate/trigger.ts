import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever } from "../error/error.ts";
import { condition, quote } from "../dialect/quote.ts";
import type { AggregateDescription } from "../inspect/aggregate.ts";
import { LOG_REPLICA } from "../log/schema.ts";
import { boundedName } from "../table/namespace.ts";

/** Generate the triggers keeping an aggregate current. */
function install(aggregate: AggregateDescription, dialect: Dialect): string[] {
    // name the triggers and skip replica copies
    const prefix = triggerPrefix(aggregate);
    const idle = `NOT EXISTS (SELECT 1 FROM ${quote(LOG_REPLICA)})`;

    // adjust counts and sums by difference, and recompute extremes
    if (dialect === "sqlite") {
        return [
            `CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${quote(aggregate.source)}
                WHEN ${idle} BEGIN ${adjust(aggregate, "NEW", 1, dialect)} END`,
            `CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${quote(aggregate.source)}
                WHEN ${idle} BEGIN ${adjust(aggregate, "OLD", -1, dialect)} END`,
            `CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${quote(aggregate.source)}
                WHEN ${idle} AND (${changed(aggregate, "IS NOT")}) BEGIN
                    ${adjust(aggregate, "OLD", -1, dialect)}
                    ${adjust(aggregate, "NEW", 1, dialect)}
                END`,
        ];
    }
    // keep them through one PostgreSQL function
    else if (dialect === "postgresql") {
        return [
            `CREATE OR REPLACE FUNCTION ${quote(`${prefix}_maintain`)}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF EXISTS (SELECT 1 FROM ${quote(LOG_REPLICA)}) THEN RETURN NULL; END IF;
                IF TG_OP = 'UPDATE' AND NOT (${changed(aggregate, "IS DISTINCT FROM")}) THEN RETURN NULL; END IF;
                IF TG_OP <> 'INSERT' THEN ${adjust(aggregate, "OLD", -1, dialect)} END IF;
                IF TG_OP <> 'DELETE' THEN ${adjust(aggregate, "NEW", 1, dialect)} END IF;
                RETURN NULL;
            END $$`,
            `CREATE TRIGGER ${quote(`${prefix}_maintain`)}
                AFTER INSERT OR UPDATE OR DELETE ON ${quote(aggregate.source)}
                FOR EACH ROW EXECUTE FUNCTION ${quote(`${prefix}_maintain`)}()`,
        ];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Remove an aggregate's triggers. */
function remove(aggregate: AggregateDescription, dialect: Dialect): string[] {
    const prefix = triggerPrefix(aggregate);

    // drop each SQLite trigger
    if (dialect === "sqlite") {
        return ["insert", "delete", "update"].map(
            (suffix) => `DROP TRIGGER IF EXISTS ${quote(`${prefix}_${suffix}`)}`,
        );
    }
    // drop the PostgreSQL function with its trigger
    else if (dialect === "postgresql") {
        return [`DROP FUNCTION IF EXISTS ${quote(`${prefix}_maintain`)}() CASCADE`];
    }
    // reject other dialects
    else {
        return assertNever(dialect);
    }
}

/** Compute an aggregate for every holding row. */
export function recomputeAggregate(aggregate: AggregateDescription, dialect: Dialect): string {
    return `UPDATE ${quote(aggregate.table)} SET ${quote(aggregate.column)} = (${computed(aggregate, dialect)})`;
}

/** Adjust the aggregate of the row a changed row references. */
function adjust(
    aggregate: AggregateDescription,
    row: "OLD" | "NEW",
    sign: 1 | -1,
    dialect: Dialect,
): string {
    // adjust only the referenced row
    const holding = `${quote(aggregate.table)}.${quote(aggregate.id)} = ${row}.${quote(aggregate.key)}`;
    const matching = aggregate.where
        .map((entry) => `${row}.${quote(entry.column)} ${condition(entry.value, dialect)}`)
        .join(" AND ");
    const guard = matching === "" ? "" : ` AND ${matching}`;
    const column = quote(aggregate.column);

    // count by one and sum by the value's difference
    if (aggregate.function === "count" || aggregate.function === "sum") {
        const amount =
            aggregate.function === "count" ? "1" : `coalesce(${row}.${quote(aggregate.value!)}, 0)`;

        return `UPDATE ${quote(aggregate.table)} SET ${column} = ${column} ${sign > 0 ? "+" : "-"} ${amount} WHERE ${holding}${guard};`;
    }

    // recompute extremes
    return `UPDATE ${quote(aggregate.table)} SET ${column} = (${computed(aggregate, dialect)}) WHERE ${holding};`;
}

/** Select an aggregate of the rows referencing a holding row. */
function computed(aggregate: AggregateDescription, dialect: Dialect): string {
    // aggregate the matching referencing rows
    const source = quote(aggregate.source);
    const expression =
        aggregate.function === "count"
            ? "count(*)"
            : aggregate.function === "sum"
              ? `coalesce(sum(${source}.${quote(aggregate.value!)}), 0)`
              : `${aggregate.function}(${source}.${quote(aggregate.value!)})`;
    const matching = aggregate.where.map(
        (entry) => ` AND ${source}.${quote(entry.column)} ${condition(entry.value, dialect)}`,
    );

    return `SELECT ${expression} FROM ${source} WHERE ${source}.${quote(aggregate.key)} = ${quote(aggregate.table)}.${quote(aggregate.id)}${matching.join("")}`;
}

/** The longest suffix an aggregate trigger name takes. */
const SUFFIX = "_maintain";

/** Build the names of an aggregate's triggers with room for their suffixes. */
function triggerPrefix(aggregate: AggregateDescription): string {
    return boundedName(
        `${aggregate.source}__${aggregate.table}_${aggregate.column}`,
        SUFFIX.length,
    );
}

/** Test whether an update changed a column the aggregate reads. */
function changed(aggregate: AggregateDescription, operator: "IS NOT" | "IS DISTINCT FROM"): string {
    return [
        aggregate.key,
        ...(aggregate.value ? [aggregate.value] : []),
        ...aggregate.where.map((entry) => entry.column),
    ]
        .map((column) => `OLD.${quote(column)} ${operator} NEW.${quote(column)}`)
        .join(" OR ");
}

/** The triggers keeping aggregates current. */
export const aggregateTriggers: Triggers = {
    install: (state, dialect) =>
        (state.aggregates ?? []).flatMap((aggregate) => install(aggregate, dialect)),
    remove: (state, dialect) =>
        (state.aggregates ?? []).flatMap((aggregate) => remove(aggregate, dialect)),
};
