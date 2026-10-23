import {
    sql,
    eq,
    ne,
    lt,
    lte,
    gt,
    gte,
    isNull,
    isNotNull,
    dialectSQL,
    TABLE,
    type Column,
    type Table,
    combine,
} from "@destack/db";
import type { CleanedWhere } from "better-auth/adapters";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Better Auth's where clauses as SQL over the account tables. */
export const BetterAuthWhere = {
    /** Resolve a declared authentication column. */
    column,

    /** Combine Better Auth's conjunction and disjunction groups. */
    condition,
};

/** Resolve a declared authentication column. */
function column(source: Table, name: string): Column {
    // find the column by its property
    const columns = source[TABLE].columns;
    const match = Object.hasOwn(columns, name) ? columns[name] : undefined;
    if (match === undefined) {
        throw AccountError.of(
            "UNKNOWN_AUTHENTICATION_COLUMN",
            `unknown authentication column: ${name}`,
        );
    }

    return match;
}

/** Combine Better Auth's conjunction and disjunction groups. */
function condition(source: Table, where: CleanedWhere[] = []) {
    // split the clauses into the conjunction and disjunction groups
    const conjunction = where
        .filter((entry) => entry.connector !== "OR")
        .map((entry) => comparison(source, entry));
    const disjunction = where
        .filter((entry) => entry.connector === "OR")
        .map((entry) => comparison(source, entry));

    // join the disjunction as one clause of the conjunction
    const clauses = [
        ...conjunction,
        ...(disjunction.length === 0 ? [] : [combine(disjunction, "OR")]),
    ];

    return clauses.length === 0 ? undefined : combine(clauses, "AND");
}

/** Compare a column with a bound authentication value. */
function comparison(source: Table, where: CleanedWhere) {
    // match strings case insensitively when requested
    const field = column(source, where.field);
    const isInsensitive =
        where.mode === "insensitive" &&
        (typeof where.value === "string" ||
            (Array.isArray(where.value) &&
                where.value.every((value) => typeof value === "string")));
    const left = isInsensitive ? sql`lower(${field})` : field;
    const value =
        isInsensitive && typeof where.value === "string" ? where.value.toLowerCase() : where.value;

    // retain null and set comparisons as SQL operations
    switch (where.operator ?? "eq") {
        // compare equality, with null as a null check
        case "eq":
            return value === null ? isNull(left) : eq(left, value);
        case "ne":
            return value === null ? isNotNull(left) : ne(left, value);
        // compare order
        case "lt":
            return lt(left, value);
        case "lte":
            return lte(left, value);
        case "gt":
            return gt(left, value);
        case "gte":
            return gte(left, value);
        // match membership in a list, with an empty list matching nothing
        case "in":
        case "not_in": {
            // refuse a value other than a list
            if (!Array.isArray(value)) {
                throw AccountError.of(
                    "OPERATOR_TAKES_LIST",
                    `authentication operator ${where.operator} takes a list`,
                );
            }

            // bind each item, lowered when case insensitive
            const values = value.map(
                (item) =>
                    sql`${isInsensitive && typeof item === "string" ? item.toLowerCase() : item}`,
            );
            const membership =
                values.length === 0 ? sql`false` : sql`${left} IN (${sql.join(values, sql`, `)})`;

            return where.operator === "in" ? membership : sql`NOT (${membership})`;
        }
        // match a substring in the dialect's function
        case "contains":
            return dialectSQL({
                sqlite: sql`instr(${left}, ${value}) > 0`,
                postgresql: sql`strpos(${left}, ${value}) > 0`,
            });
        // match a prefix or suffix
        case "starts_with":
            return sql`substr(${left}, 1, length(${value})) = ${value}`;
        case "ends_with":
            return sql`substr(${left}, length(${left}) - length(${value}) + 1) = ${value}`;
        // refuse unknown operators
        default:
            throw AccountError.of(
                "UNKNOWN_AUTHENTICATION_OPERATOR",
                `unknown authentication operator: ${String(where.operator)}`,
            );
    }
}
