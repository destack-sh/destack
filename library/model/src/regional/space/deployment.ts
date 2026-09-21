import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { WorkloadDescription } from "@destack/package/workload";
import { serviceAccount } from "../access/service.ts";
import { Conditions } from "../../record/index.ts";
import { installation } from "./installation.ts";
import { spaceHost } from "./host.ts";
import { DeploymentPolicies } from "../access/deployment.ts";

/** A prepared installation generation for one workload and build output. */
export const deployment = table(
    "deployment",
    {
        ...recordColumns("deployment"),
        /** The installation's space. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The persistent package installation. */
        installationId: identifier("installation_id", "installation").notNull(),
        /** The installation generation captured by this deployment. */
        generation: integer("generation").notNull(),
        /** The installed package. */
        packageId: identifier("package_id", "package").notNull(),
        /** The immutable release selected for this deployment. */
        version: text("version").notNull(),
        /** The exact build manifest digest. */
        manifest: text("manifest").notNull(),
        /** The named build output. */
        output: text("output").notNull(),
        /** The package-local workload name. */
        workload: text("workload").notNull(),
        /** The runtime of the selected build output. */
        runtime: text("runtime", { enum: ["bun", "workerd"] }).notNull(),
        /** The installation workload identity. */
        serviceAccountId: identifier("service_account_id", "service-account").notNull(),
        /** The workload description and effective compute settings captured at preparation. */
        description: json("description", WorkloadDescription).notNull(),
        /** Exact policies checked at preparation and rechecked against current revisions at activation. */
        policies: json("policies", DeploymentPolicies).notNull(),
        /** An explicitly selected execution host, absent for scheduling across space hosts. */
        hostId: identifier("host_id", "host"),
        /** The desired deployment availability. */
        state: text("state", { enum: ["prepared", "active", "draining", "retired"] })
            .notNull()
            .default("prepared"),
        /** Controller observations, including readiness and deployment failures. */
        conditions: json("conditions", Conditions)
            .notNull()
            .default(sql`'{}'`),
        /** Activation time in UTC epoch milliseconds. */
        activatedAt: integer("activated_at"),
        /** Retirement time in UTC epoch milliseconds. */
        retiredAt: integer("retired_at"),
    },
    (entry) => [
        unique("deployment_space_id").on(entry.spaceId, entry.id),
        unique("deployment_service_account").on(entry.id, entry.serviceAccountId),
        unique("deployment_generation").on(
            entry.installationId,
            entry.generation,
            entry.output,
            entry.workload,
        ),
        foreignKey({
            columns: [entry.spaceId, entry.installationId, entry.packageId],
            foreignColumns: [installation.spaceId, installation.id, installation.packageId],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.spaceId, entry.hostId],
            foreignColumns: [spaceHost.spaceId, spaceHost.hostId],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.installationId, entry.workload, entry.serviceAccountId],
            foreignColumns: [
                serviceAccount.installationId,
                serviceAccount.workload,
                serviceAccount.id,
            ],
        }).onDelete("restrict"),
        index("deployment_installation_state").on(entry.installationId, entry.state),
        check("deployment_runtime", sql`${entry.runtime} IN ('bun', 'workerd')`),
        check("deployment_workload", sql`length(${entry.workload}) > 0`),
        check("deployment_generation_positive", sql`${entry.generation} > 0`),

        check(
            "deployment_state",
            sql`${entry.state} IN ('prepared', 'active', 'draining', 'retired')`,
        ),
        check(
            "deployment_times",
            sql`${entry.retiredAt} IS NULL OR ${entry.activatedAt} IS NULL OR ${entry.retiredAt} >= ${entry.activatedAt}`,
        ),
    ],
);

/** A persisted deployment. */
export type Deployment = Select<typeof deployment>;
