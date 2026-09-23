import { defineDatabaseSchema } from "@destack/db";
import { auditOutboxSchema } from "@destack/audit/outbox";
import { auditSchema } from "@destack/audit/history";
import { host } from "./host.ts";
import { checkout } from "./checkout.ts";
import { login } from "./login.ts";
import { deployment } from "./deployment.ts";
import { instance } from "./instance.ts";

/** Host records and durable audit in daemon.db. */
export const daemonSchema = defineDatabaseSchema({
    name: "host",
    tables: { host, checkout, login, deployment, instance },
    dependencies: [auditOutboxSchema, auditSchema],
    migrations: new URL("./migration/", import.meta.url),
});
