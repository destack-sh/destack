import { principal } from "@destack/access";
import { type Select, type Condition, type RowImage } from "@destack/db";
import { SERVER_RUNTIMES } from "@destack/package/runtime";
import { type ObjectOf, ConditionMap, defineObject, field } from "@destack/object";
import { DirectoryGrant, PackageId } from "@destack/package";
import { WorkloadDescription } from "@destack/package/manifest";
import { schema, Version } from "@destack/schema";
import { installation } from "@destack/object";
import { installationRevision } from "./rollout.ts";
import { space } from "@destack/account/object";

/** The runtime of a deployment the machine serves in its own process: a shipped server's. */
export const PROCESS_RUNTIME = "process";

/** The runtimes a deployment runs on: a server runtime, or the machine's own process. */
export const DEPLOYMENT_RUNTIMES = [...SERVER_RUNTIMES, PROCESS_RUNTIME] as const;

/** A runtime a deployment runs on. */
export type DeploymentRuntime = (typeof DEPLOYMENT_RUNTIMES)[number];

/** One rollout of a workload of an installation revision from one build output. */
export const deployment = defineObject(
    {
        name: "deployment",
        plural: "deployments",
        scope: space,
        fields: {
            /** The persistent package installation. */
            installation: field.reference(
                (): ObjectOf<{ identity: "installation" }> => installation,
            ),
            /** The installed package. */
            packageId: field.string(PackageId),
            /** The installation's revision whose build the deployment runs. */
            revision: field.reference(
                (): ObjectOf<{ identity: "installation-revision" }> => installationRevision,
            ),
            /** The named build output. */
            output: field.string(),
            /** The package-local workload name. */
            workload: field.string(),
            /** The runtime of the selected build output. */
            runtime: field.enum(DEPLOYMENT_RUNTIMES),
            /** The package release the build runs. */
            release: field.string(Version),
            /** The oldest caller release the package serves. */
            since: field.string(Version).optional(),
            /** The workload description and effective compute settings captured at preparation. */
            description: field.json(WorkloadDescription),
            /** The directories the workload's `fs` capability was granted at preparation. */
            directories: field.json(schema.array(DirectoryGrant)).default([]),
            /** The package's capabilities the workload runs with, enabled at preparation. */
            enabled: field.json(schema.array(schema.string().min(1))).default([]),
            /** An explicitly selected machine running the workload. */
            machine: field.reference(principal.machine).optional(),
            /** The desired deployment availability. */
            status: field.enum(["active", "draining", "stopping", "retired"]).default("active"),
            /** Controller observations, including readiness and deployment failures. */
            conditions: field.json(ConditionMap).default({}),
            /** Activation time in UTC epoch milliseconds, as every deployment starts active. */
            activatedAt: field.time(),
            /** Retirement time in UTC epoch milliseconds, absent until retired. */
            retiredAt: field.time().optional(),
        },
        permissions: ["list", "read", "record"],
        constraints: {
            // key deployments by space and allow one active per workload
            scopeId: { kind: "unique", on: ["scope", "id"] },
            workload: {
                kind: "index",
                on: ["installationId", "workload"],
                unique: true,
                where: { status: "active" },
            },

            // relate each deployment to its installation and index it by status
            installation: {
                kind: "foreignKey",
                on: ["scope", "installationId", "packageId"],
                references: { object: () => installation, on: ["scope", "id", "packageId"] },
                onDelete: "no action",
            },
            installationStatus: { kind: "index", on: ["installationId", "status"] },

            // check the workload and the retirement time
            workloadForm: { kind: "check", condition: { workload: { length: { gt: 0 } } } },
            times: {
                kind: "check",
                condition: {
                    retiredAt: { OR: [{ isNull: true }, { gte: { field: "activatedAt" } }] },
                },
            },
            retired: {
                kind: "check",
                condition: {
                    OR: [
                        { status: "retired", retiredAt: { isNotNull: true } },
                        { status: { ne: "retired" }, retiredAt: { isNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("record", { visibility: "package" }),
        update: method.update("record", { visibility: "package" }),
        delete: method.delete("record", { visibility: "package" }),
    }),
);

/** A persisted deployment. */
export type Deployment = Select<typeof deployment.table>;

/** Reads and calls of deployments that the packages serving their workloads share. */
export const Deployment = {
    /** Drain a deployment: it stops taking new work and finishes what it has. */
    drain(target: Pick<Deployment, "id" | "scope">) {
        return deployment.call("update", { ...deployment.target(target), status: "draining" });
    },

    /** Match the deployments whose workloads run. */
    live(): Condition<RowImage<typeof deployment.table>> {
        return { status: { in: ["active", "draining", "stopping"] } };
    },

    /** Match the deployments available to serve requests. */
    available(): Condition<RowImage<typeof deployment.table>> {
        return { status: { in: ["active", "draining"] } };
    },
};
