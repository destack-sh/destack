import { defineSchema, schema } from "@destack/schema";
import { Dialect } from "../dialect/dialect.ts";
import { COLUMN_KINDS, REFERENCE_ACTIONS } from "../table/column.ts";

/** A column as the database stores it. */
export const ColumnDescription = defineSchema(
    schema.object({
        /** The SQL column name. */
        name: schema.string(),
        /** The logical value type. */
        kind: schema.enum(COLUMN_KINDS),
        /** The JSON Schema of the column's values. */
        value: schema.record(schema.string(), schema.json()),
        /** The dialect-specific SQL type. */
        type: schema.string(),
        /** Whether the declaration allows NULL. */
        nullable: schema.boolean(),
        /** The SQL default expression. */
        default: schema.string().exactOptional(),
        /** The generated column expression and storage mode. */
        generated: schema
            .object({
                /** The storage mode, the dialect default when absent. */
                mode: schema.enum(["virtual", "stored"]).exactOptional(),
                /** The SQL generation expression. */
                expression: schema.string(),
            })
            .exactOptional(),
    }),
);
/** A column as the database stores it. */
export type ColumnDescription = schema.Infer<typeof ColumnDescription>;

/** A named table constraint as the database stores it. */
export const ConstraintDescription = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A primary key or unique constraint. */
            kind: schema.enum(["primaryKey", "unique"]),
            /** The SQL constraint name. */
            name: schema.string(),
            /** The constrained columns in declaration order. */
            columns: schema.array(schema.string()),
        }),
        schema.object({
            /** A foreign key. */
            kind: schema.literal("foreignKey"),
            /** The SQL constraint name. */
            name: schema.string(),
            /** The local column names. */
            columns: schema.array(schema.string()),
            /** The referenced table name. */
            table: schema.string(),
            /** The referenced column names in matching order. */
            references: schema.array(schema.string()),
            /** The SQL action when a referenced key changes. */
            onUpdate: schema.enum(REFERENCE_ACTIONS).exactOptional(),
            /** The SQL action when a referenced row is deleted. */
            onDelete: schema.enum(REFERENCE_ACTIONS).exactOptional(),
        }),
        schema.object({
            /** A check. */
            kind: schema.literal("check"),
            /** The SQL constraint name. */
            name: schema.string(),
            /** The SQL check expression. */
            expression: schema.string(),
        }),
    ]),
);
/** A named table constraint as the database stores it. */
export type ConstraintDescription = schema.Infer<typeof ConstraintDescription>;

/** An index as the database stores it. */
export const IndexDescription = defineSchema(
    schema.object({
        /** The SQL index name. */
        name: schema.string(),
        /** Whether the index enforces uniqueness. */
        unique: schema.boolean(),
        /** Indexed columns and expressions in index order. */
        columns: schema.array(
            schema.union([
                schema.object({ column: schema.string() }),
                schema.object({ expression: schema.string() }),
            ]),
        ),
        /** The SQL predicate of a partial index. */
        where: schema.string().exactOptional(),
    }),
);
/** An index as the database stores it. */
export type IndexDescription = schema.Infer<typeof IndexDescription>;

/** A table's columns, constraints and indexes. */
export const TableDescription = defineSchema(
    schema.object({
        /** The SQL dialect. */
        dialect: Dialect,
        /** The SQL table name. */
        name: schema.string(),
        /** Columns in declaration order. */
        columns: schema.array(ColumnDescription),
        /** Primary keys, unique constraints, foreign keys and checks. */
        constraints: schema.array(ConstraintDescription),
        /** Explicit indexes, including expression and partial indexes. */
        indexes: schema.array(IndexDescription),
    }),
);
/** A table's columns, constraints and indexes. */
export type TableDescription = schema.Infer<typeof TableDescription>;
