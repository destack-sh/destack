import { table, text, defineDatabaseSchema } from "@destack/db";

/** Application state changed in the same transaction as its audit event. */
export const document = table("document", { name: text("name").primaryKey().notNull() });

/** Application tables used by the audit integration scenarios. */
export const testSchema = defineDatabaseSchema({
    name: "audit-integration",
    tables: { document },
    migrations: new URL("./migration/", import.meta.url),
});
