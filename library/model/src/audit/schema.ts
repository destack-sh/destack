import { defineDatabaseSchema } from "@destack/db";
import { auditEvent } from "./event.ts";

/** Audit records with an independent migration history. */
export const auditSchema = defineDatabaseSchema({
    name: "destack-audit",
    tables: { auditEvent },
    migrations: new URL("./migration/", import.meta.url),
});
