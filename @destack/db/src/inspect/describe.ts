import { inline, SQL } from "../sql/index.ts";
import { schema, toJsonSchema } from "@destack/schema";
import { Column } from "../table/column.ts";
import type { ForeignKey, Index, TableConstraint } from "../table/constraint.ts";
import { TABLE, type Table } from "../table/table.ts";
import {
    type ColumnDescription,
    type ConstraintDescription,
    type IndexDescription,
    TableDescription,
} from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { boundedName, qualify } from "../table/namespace.ts";
import { literal, quote } from "../dialect/quote.ts";

/** The PostgreSQL collation comparing text by its UTF-8 bytes, as SQLite's default does. */
const BYTE_COLLATION = "C";

/** A primary key or unique constraint over some columns. */
interface UniqueKey {
    /** The constraint category. */
    readonly kind: "primaryKey" | "unique";
    /** The declared name. */
    readonly name: string | undefined;
    /** The constrained columns. */
    readonly columns: readonly Column[];
}

/** Describe a table in a dialect. */
export function describeTable(table: Table, dialect: Dialect): TableDescription {
    // collect the declared constraints
    const definition = table[TABLE];
    const constraints = definition.constraints(dialect);
    const columns = Object.values(definition.columns);

    return {
        dialect,
        name: definition.sqlName,
        columns: columns.map((column) => describeColumn(column, dialect)),
        constraints: [
            ...uniqueKeys(table, constraints).map((key) => describeKey(table, key)),
            ...constraints
                .filter((value) => value.kind === "foreignKey")
                .map((key) => describeForeignKey(table, key)),
            ...constraints
                .filter((value) => value.kind === "check")
                .map((check) => ({
                    kind: "check" as const,
                    name: check.name,
                    expression: inline(check.expression, dialect),
                })),
            ...columns.flatMap((column) => describeEnum(table, column)),
        ],
        indexes: constraints
            .filter((value) => value.kind === "index")
            .map((index) => describeIndex(table, index, dialect)),
    };
}

/** Describe a column. */
function describeColumn(column: Column, dialect: Dialect): ColumnDescription {
    const definition = column.definition;

    return {
        name: definition.name,
        kind: definition.kind,
        value: schema.record(schema.string(), schema.json()).parse(toJsonSchema(definition.json)),
        type: definition.types[dialect],
        ...(dialect === "postgresql" && definition.types.postgresql === "text"
            ? { collation: BYTE_COLLATION }
            : {}),
        nullable: definition.nullable,
        ...(definition.default === undefined
            ? {}
            : {
                  default: inline(
                      definition.default instanceof SQL
                          ? definition.default
                          : definition.encode(definition.default, dialect),
                      dialect,
                  ),
              }),
        ...(definition.generated === undefined
            ? {}
            : {
                  generated: {
                      mode: definition.generated.mode,
                      expression: inline(
                          typeof definition.generated.expression === "function"
                              ? definition.generated.expression()
                              : definition.generated.expression,
                          dialect,
                      ),
                  },
              }),
    };
}

/** Gather a table's primary key and unique constraints. */
function uniqueKeys(table: Table, constraints: readonly TableConstraint[]): UniqueKey[] {
    const definition = table[TABLE];

    return [
        ...(definition.key.length === 0
            ? []
            : [
                  {
                      kind: "primaryKey" as const,
                      name: undefined,
                      columns: definition.key.map((property) => definition.column(property)),
                  },
              ]),
        ...Object.values(definition.columns).flatMap((column) => {
            const unique = column.definition.unique;

            return unique === undefined
                ? []
                : [{ kind: "unique" as const, name: unique.name, columns: [column] }];
        }),
        ...constraints.flatMap((constraint) =>
            constraint.kind === "unique"
                ? [{ kind: "unique" as const, name: constraint.name, columns: constraint.columns }]
                : [],
        ),
    ];
}

/** Describe a primary key or unique constraint. */
function describeKey(table: Table, key: UniqueKey): ConstraintDescription {
    const definition = table[TABLE];

    return {
        kind: key.kind,
        name:
            (key.name === undefined ? undefined : qualify(definition.package, key.name)) ??
            derivedName(
                definition.sqlName,
                key.columns,
                key.kind === "primaryKey" ? "pk" : "unique",
            ),
        columns: key.columns.map((column) => column.definition.name),
    };
}

/** Describe a foreign key. */
function describeForeignKey(table: Table, key: ForeignKey): ConstraintDescription {
    const foreignColumns = key.foreignColumns.map((column) => column.definition.name);

    return {
        kind: "foreignKey",
        name:
            key.name ??
            derivedName(
                table[TABLE].sqlName,
                key.columns,
                `${key.references}_${foreignColumns.join("_")}_fk`,
            ),
        columns: key.columns.map((column) => column.definition.name),
        table: key.references,
        references: foreignColumns,
        ...(key.actions.onDelete === undefined ? {} : { onDelete: key.actions.onDelete }),
        ...(key.actions.onUpdate === undefined ? {} : { onUpdate: key.actions.onUpdate }),
    };
}

/** Describe the check limiting an enum column to its values. */
function describeEnum(table: Table, column: Column): ConstraintDescription[] {
    const values = column.definition.enumValues;

    return values === undefined
        ? []
        : [
              {
                  kind: "check",
                  name: derivedName(table[TABLE].sqlName, [column], "enum"),
                  expression: `${quote(column.definition.name)} IN (${values.map(literal).join(", ")})`,
              },
          ];
}

/** Describe an index over columns or expressions. */
function describeIndex(table: Table, index: Index, dialect: Dialect): IndexDescription {
    return {
        name: qualify(table[TABLE].package, index.name),
        unique: index.isUnique,
        columns: index.columns.map((column) =>
            column instanceof Column
                ? { column: column.definition.name }
                : { expression: inline(column, dialect) },
        ),
        ...(index.predicate === undefined ? {} : { where: inline(index.predicate, dialect) }),
    };
}

/** Derive a constraint name within the identifier limit. */
function derivedName(table: string, columns: readonly Column[], suffix: string): string {
    return boundedName(
        [table, ...columns.map((column) => column.definition.name), suffix].join("_"),
    );
}
