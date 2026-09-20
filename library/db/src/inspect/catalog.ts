import { defineSchema, schema } from "@destack/schema";
import { Dialect } from "../dialect/dialect.ts";

/** A persisted database schema object. */
export const DatabaseObject = defineSchema(schema.object({
    /** The database object type. */
    kind: schema.enum(["table", "index", "view", "materializedView", "sequence", "trigger"]),
    /** The SQL namespace, when the database supports namespaces. */
    schema: schema.string().optional(),
    /** The object's SQL name. */
    name: schema.string().min(1),
    /** The associated table or view; standalone objects use their own name. */
    table: schema.string().min(1),
    /** SQL supplied by the catalog; null when no definition is available. */
    sql: schema.string().nullable(),
}));
/** A persisted database schema object. */
export type DatabaseObject = schema.Infer<typeof DatabaseObject>;

/** An inventory of persisted database objects. */
export const DatabaseCatalog = defineSchema(schema.object({
    /** The inspected SQL dialect. */
    dialect: Dialect,
    /** Schema objects in stable order. */
    objects: schema.array(DatabaseObject),
}));
/** An inventory of persisted database objects. */
export type DatabaseCatalog = schema.Infer<typeof DatabaseCatalog>;
