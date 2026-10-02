import { inline, SQL } from "../sql/index.ts";
import { schema, toJsonSchema } from "@destack/schema";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { TableDescription } from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { boundedName, qualify } from "../table/namespace.ts";
import { literal, quote } from "../dialect/quote.ts";

/** Describe a table in a dialect. */
export function describeTable(table: Table, dialect: Dialect): TableDescription {
    // collect the declared constraints
    const definition = table[TABLE];
    const constraints = definition.constraints(dialect);
    const columns = Object.values(definition.columns);

    // gather primary keys and unique constraints
    const keys: {
        kind: "primaryKey" | "unique";
        name: string | undefined;
        columns: readonly Column[];
    }[] = [
        ...(definition.key.length === 0
            ? []
            : [
                  {
                      kind: "primaryKey" as const,
                      name: undefined,
                      columns: definition.key.map((property) => definition.column(property)),
                  },
              ]),
        ...columns.flatMap((column) => {
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

    return {
        dialect,
        name: definition.sqlName,
        columns: columns.map((column) => ({
            name: column.definition.name,
            kind: column.definition.kind,
            value: schema
                .record(schema.string(), schema.json())
                .parse(toJsonSchema(column.definition.json ?? column.definition.schema)),
            type: column.definition.types[dialect],
            nullable: column.definition.nullable,
            ...(column.definition.default === undefined
                ? {}
                : {
                      default: inline(
                          column.definition.default instanceof SQL
                              ? column.definition.default
                              : column.definition.encode(column.definition.default, dialect),
                          dialect,
                      ),
                  }),
            ...(column.definition.generated === undefined
                ? {}
                : {
                      generated: {
                          mode: column.definition.generated.mode,
                          expression: inline(
                              typeof column.definition.generated.expression === "function"
                                  ? column.definition.generated.expression()
                                  : column.definition.generated.expression,
                              dialect,
                          ),
                      },
                  }),
        })),
        constraints: [
            ...keys.map((key) => ({
                kind: key.kind,
                name:
                    (key.name === undefined ? undefined : qualify(definition.package, key.name)) ??
                    derivedName(
                        definition.sqlName,
                        key.columns,
                        key.kind === "primaryKey" ? "pk" : "unique",
                    ),
                columns: key.columns.map((column) => column.definition.name),
            })),
            ...constraints
                .filter((value) => value.kind === "foreignKey")
                .map((key) => ({
                    kind: "foreignKey" as const,
                    name:
                        key.name ??
                        derivedName(
                            definition.sqlName,
                            key.columns,
                            `${key.references}_${key.foreignColumns
                                .map((column) => column.definition.name)
                                .join("_")}_fk`,
                        ),
                    columns: key.columns.map((column) => column.definition.name),
                    table: key.references,
                    references: key.foreignColumns.map((column) => column.definition.name),
                    ...(key.actions.onDelete === undefined
                        ? {}
                        : { onDelete: key.actions.onDelete }),
                    ...(key.actions.onUpdate === undefined
                        ? {}
                        : { onUpdate: key.actions.onUpdate }),
                })),
            ...constraints
                .filter((value) => value.kind === "check")
                .map((check) => ({
                    kind: "check" as const,
                    name: check.name,
                    expression: inline(check.expression, dialect),
                })),
            ...columns.flatMap((column) => {
                const values = column.definition.enumValues;

                return values === undefined
                    ? []
                    : [
                          {
                              kind: "check" as const,
                              name: derivedName(definition.sqlName, [column], "enum"),
                              expression: `${quote(column.definition.name)} IN (${values.map(literal).join(", ")})`,
                          },
                      ];
            }),
        ],
        indexes: constraints
            .filter((value) => value.kind === "index")
            .map((index) => ({
                name: qualify(definition.package, index.name),
                unique: index.isUnique,
                columns: index.columns.map((column) =>
                    column instanceof Column
                        ? { column: column.definition.name }
                        : { expression: inline(column, dialect) },
                ),
                ...(index.predicate === undefined
                    ? {}
                    : { where: inline(index.predicate, dialect) }),
            })),
    };
}

/** Derive a constraint name within the identifier limit. */
function derivedName(table: string, columns: readonly Column[], suffix: string): string {
    return boundedName(
        [table, ...columns.map((column) => column.definition.name), suffix].join("_"),
    );
}
