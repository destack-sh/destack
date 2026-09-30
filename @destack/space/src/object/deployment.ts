import {
    check,
    foreignKey,
    index,
    inArray,
    sql,
    unique,
    uniqueIndex,
    type Select,
    type SQL,
} from "@destack/db";
import { ConditionMap, defineObject, field, method, type ObjectType } from "@destack/object";
import { PackageId } from "@destack/package";
import { ServerRuntime } from "@destack/package/runtime";
import { WorkloadDescription } from "@destack/package/workload";
import { identifier, schema, Version } from "@destack/schema";
import { PolicySelection } from "../policy/selection.ts";
import { installation, installationRevision } from "./installation.ts";
import { space } from "./space.ts";

/** Record a new instance of a deployment starting on a host, as the cell's instance controller decides. */
const start = method({
    permission: null,
    isSystem: true,
    input: schema.object({
        /** The host starting the instance. */
        hostId: identifier("host"),
    }),
});

/** One rollout of a workload of an installation revision from one build output; at most one per workload is active. */
export const deployment = defineObject({
    name: "deployment",
    plural: "deployments",
    scope: space,
    fields: {
        /** The persistent package installation. */
        installationId: field.reference<"installation">((): ObjectType => installation),
        /** The installed package. */
        packageId: field.string(PackageId),
        /** The installation's revision whose build the deployment runs. */
        revisionId: field.reference<"installation-revision">(
            (): ObjectType => installationRevision,
        ),
        /** The named build output. */
        output: field.string(),
        /** The package-local workload name. */
        workload: field.string(),
        /** The runtime of the selected build output. */
        runtime: field.enum(ServerRuntime.options as [ServerRuntime, ...ServerRuntime[]]),
        /** The package release the build runs. */
        release: field.string(Version),
        /** The oldest caller release the package's service serves, every earlier one when absent. */
        since: field.string(Version).optional(),
        /** The workload description and effective compute settings captured at preparation. */
        description: field.json(WorkloadDescription),
        /** Exact policies checked at preparation and rechecked against current revisions at activation. */
        policies: field.json(PolicySelection),
        /** An explicitly selected execution host, absent for scheduling across space hosts. */
        hostId: field.string(identifier("host")).optional(),
        /** The desired deployment availability. */
        status: field.enum(["active", "draining", "stopping", "retired"]).default("active"),
        /** Controller observations, including readiness and deployment failures. */
        conditions: field.json(ConditionMap).default({}),
        /** Activation time in UTC epoch milliseconds. */
        activatedAt: field.time().optional(),
        /** Retirement time in UTC epoch milliseconds. */
        retiredAt: field.time().optional(),
    },
    constraints: (entry) => [
        unique("deployment_scope_id").on(entry.scope, entry.id),
        uniqueIndex("deployment_workload")
            .on(entry.installationId, entry.workload)
            .where(sql`${entry.status} = 'active'`),
        foreignKey({
            columns: [entry.scope, entry.installationId, entry.packageId],
            foreignColumns: [
                installation.table.scope,
                installation.table.id,
                installation.table.packageId,
            ],
        }).onDelete("restrict"),
        index("deployment_installation_status").on(entry.installationId, entry.status),
        check("deployment_workload", sql`length(${entry.workload}) > 0`),
        check(
            "deployment_times",
            sql`${entry.retiredAt} IS NULL OR ${entry.activatedAt} IS NULL OR ${entry.retiredAt} >= ${entry.activatedAt}`,
        ),
    ],
    permissions: ["list", "read"],
    methods: {
        get: method.get("read"),
        list: method.list("list"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
        start,
    },
});

/** A persisted deployment. */
export type Deployment = Select<typeof deployment.table>;

/** Reads of deployments that the packages serving their workloads share. */
export const Deployment = {
    /** Match the deployments whose workloads run: active ones, draining ones still serving, and stopping ones. */
    live(): SQL {
        return inArray(deployment.table.status, ["active", "draining", "stopping"]);
    },
    /** Match the deployments whose workloads serve: active ones, and draining ones until their replacements run. */
    serving(): SQL {
        return inArray(deployment.table.status, ["active", "draining"]);
    },
};
