import { defineSchema, schema } from "@destack/schema";

/** A named constraint over table columns. */
const Constraint = schema.object({
    /** The SQL constraint name. */
    name: schema.string(),
    /** The constrained columns in declaration order. */
    columns: schema.array(schema.string()),
});

/** A declared SQL column. */
export const ColumnDescription = defineSchema(schema.object({
    /** The property name used by application code. */
    property: schema.string(),
    /** The SQL column name. */
    name: schema.string(),
    /** The dialect-specific SQL type. */
    type: schema.string(),
    /** The application value type reported by the column adapter. */
    dataType: schema.string(),
    /** The adapter mode, including timestamp units. */
    mode: schema.string().optional(),
    /** Values declared by a typed enum; SQL enforcement requires a constraint. */
    enumValues: schema.array(schema.string()).optional(),
    /** Whether the declaration allows NULL. */
    nullable: schema.boolean(),
    /** Whether the column declares a primary key. */
    primaryKey: schema.boolean(),
    /** Whether the column declares a unique constraint. */
    unique: schema.boolean(),
    /** The name of a column-level unique constraint. */
    uniqueName: schema.string().optional(),
    /** Whether SQLite prevents reuse of previously assigned row identifiers. */
    autoIncrement: schema.boolean(),
    /** Whether insertion can omit the value, including implicit primary key defaults. */
    hasDefault: schema.boolean(),
    /** The SQL default expression. */
    default: schema.string().optional(),
    /** Whether application code supplies a default value. */
    hasRuntimeDefault: schema.boolean(),
    /** Whether application code supplies an updated value. */
    hasRuntimeUpdate: schema.boolean(),
    /** The generated column expression and storage mode. */
    generated: schema.object({
        /** The declared storage mode; omission uses the dialect default. */
        mode: schema.enum(["virtual", "stored"]).optional(),
        /** The SQL generation expression. */
        expression: schema.string(),
    }).optional(),
    /** The JSON Schema for values accepted by a validated column. */
    jsonSchema: schema.record(schema.string(), schema.json()).optional(),
}));
/** A declared SQL column. */
export type ColumnDescription = schema.Infer<typeof ColumnDescription>;

/** A table's columns and SQL constraints. */
export const TableDescription = defineSchema(schema.object({
    /** The SQL table name. */
    name: schema.string(),
    /** Columns in declaration order. */
    columns: schema.array(ColumnDescription),
    /** Table-level primary key constraints. */
    primaryKeys: schema.array(Constraint),
    /** Table-level unique constraints. */
    uniqueConstraints: schema.array(Constraint),
    /** Explicit indexes, including expression and partial indexes. */
    indexes: schema.array(schema.object({
        /** The SQL index name. */
        name: schema.string(),
        /** Whether the index enforces uniqueness. */
        unique: schema.boolean(),
        /** Indexed columns and expressions in index order. */
        columns: schema.array(schema.union([
            schema.object({ column: schema.string() }),
            schema.object({ expression: schema.string() }),
        ])),
        /** The SQL predicate of a partial index. */
        where: schema.string().optional(),
    })),
    /** Foreign key targets and referential actions. */
    foreignKeys: schema.array(schema.object({
        /** The SQL constraint name. */
        name: schema.string(),
        /** The local column names. */
        columns: schema.array(schema.string()),
        /** The referenced table name. */
        table: schema.string(),
        /** The referenced column names in matching order. */
        references: schema.array(schema.string()),
        /** The SQL action when a referenced key changes. */
        onUpdate: schema.string().optional(),
        /** The SQL action when a referenced row is deleted. */
        onDelete: schema.string().optional(),
    })),
    /** Named SQL check expressions. */
    checks: schema.array(schema.object({ name: schema.string(), expression: schema.string() })),
}));
/** A table's columns and SQL constraints. */
export type TableDescription = schema.Infer<typeof TableDescription>;
