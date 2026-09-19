import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { account } from "../account/account.ts";
import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";
import { domain } from "./domain.ts";
import { spaceDirectory } from "../directory/space.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** Route records. */
export const route = table("route", {
    ...recordColumns("route"),
    ...provenanceColumns(),
    /** The account administering this route. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The hostname serving this route. */
    domainId: identifier("domain_id", "domain").notNull(),
    /** The absolute URL path. */
    path: text("path").notNull(),
    /** The path matching rule. */
    match: text("match", { enum: ["exact", "prefix"] }).notNull(),
    /** The destination kind. */
    kind: text("kind", { enum: ["application", "redirect"] }).notNull(),
    /** The destination space, whose access policy is checked by the routing service. */
    spaceId: identifier("space_id", "space").references(() => spaceDirectory.id, {
        onDelete: "restrict",
    }),
    /** The destination installation. */
    installationId: identifier("installation_id", "installation"),
    /** The exported application entrypoint. */
    entrypoint: text("entrypoint"),
    /** The HTTP redirect URL. */
    redirect: text("redirect"),
    /** The redirect status code. */
    redirectStatus: integer("redirect_status"),
    /** The route generation currently served by the gateway. */
    appliedGeneration: integer("applied_generation").notNull().default(0),

    ...reconciliationColumns(),
}, (route) => [
    foreignKey({
        columns: [route.accountId, route.domainId],
        foreignColumns: [domain.accountId, domain.id],
    }).onDelete("restrict"),
    ...provenanceChecks("route", route),
    ...reconciliationChecks("route", route),
    unique("route_domain_path_match").on(route.domainId, route.path, route.match),
    check("route_match", sql`${route.match} IN ('exact', 'prefix')`),
    check("route_path", sql`substr(${route.path}, 1, 1) = '/'`),
    check(
        "route_applied_generation",
        sql`${route.appliedGeneration} BETWEEN 0 AND ${route.generation}`,
    ),
    check(
        "route_destination",
        sql`(${route.kind} = 'application' AND ${route.spaceId} IS NOT NULL AND ${route.installationId} IS NOT NULL AND ${route.entrypoint} IS NOT NULL AND ${route.redirect} IS NULL AND ${route.redirectStatus} IS NULL) OR (${route.kind} = 'redirect' AND ${route.spaceId} IS NULL AND ${route.installationId} IS NULL AND ${route.entrypoint} IS NULL AND ${route.redirect} IS NOT NULL AND ${route.redirectStatus} IN (301, 302, 303, 307, 308) AND ${route.redirectStatus} IS NOT NULL)`,
    ),
]);

/** A persisted route record. */
export type Route = Select<typeof route>;
