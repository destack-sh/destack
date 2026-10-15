import { principal } from "@destack/access";
import { and, eq, type DatabaseConnection, type Select } from "@destack/db";
import { type ObjectOf, defineObject, field, StatusCondition } from "@destack/object";
import { defineSchema, schema, type Identifier, type Version } from "@destack/schema";
import type { ServerRuntime } from "@destack/package/runtime";
import { deployment } from "./deployment.ts";
import { space } from "@destack/account/object";

/** A condition of an instance, as a pod's conditions are. */
export const InstanceCondition = defineSchema(StatusCondition.omit({ observedGeneration: true }));
/** A condition of an instance. */
export type InstanceCondition = schema.Infer<typeof InstanceCondition>;

/** An observed running occurrence of a deployment on one machine. */
export const instance = defineObject({
    name: "instance",
    plural: "instances",
    scope: space,
    fields: {
        /** The immutable deployment being executed. */
        deployment: field.reference((): ObjectOf<{ identity: "deployment" }> => deployment),
        /** The machine administering this instance. */
        machine: field.reference(principal.machine),
        /** The latest observed execution status. */
        status: field.enum(["starting", "running", "stopped", "failed"]),
        /** The machine's observations and failure details, by condition name. */
        conditions: field
            .json(schema.record(schema.string().min(1), InstanceCondition))
            .default({}),
        /** The last authenticated observation in UTC epoch milliseconds. */
        observedAt: field.time(),
        /** The execution start time in UTC epoch milliseconds. */
        startedAt: field.time().optional(),
        /** The execution finish time in UTC epoch milliseconds. */
        stoppedAt: field.time().optional(),
        /** The restarts after failures since the instance was recorded. */
        restarts: field.integer().default(0),
    },
    permissions: ["read", "record"],
    constraints: {
        // relate each instance to its deployment and index it by status
        deployment: {
            kind: "foreignKey",
            on: ["scope", "deploymentId"],
            references: { object: () => deployment, on: ["scope", "id"] },
            onDelete: "no action",
        },
        deploymentStatus: { kind: "index", on: ["deploymentId", "status"] },
        machineStatus: { kind: "index", on: ["machineId", "status"] },

        // check the times each status keeps
        times: {
            kind: "check",
            condition: {
                OR: [
                    { stoppedAt: { isNull: true } },
                    { startedAt: { isNull: true } },
                    { stoppedAt: { gte: { field: "startedAt" } } },
                ],
            },
        },
        running: {
            kind: "check",
            condition: {
                OR: [
                    { status: { ne: "running" } },
                    { startedAt: { isNotNull: true }, stoppedAt: { isNull: true } },
                ],
            },
        },
        stopped: {
            kind: "check",
            condition: {
                OR: [
                    { status: { notIn: ["stopped", "failed"] } },
                    { stoppedAt: { isNotNull: true } },
                ],
            },
        },
    },
}).methods((method) => ({
    create: method.create("record", { isInternal: true }),
    update: method.update("record", { isInternal: true }),
    delete: method.delete("record", { isInternal: true }),
    /** Record an instance stopped, as its machine's instance controller decides. */
    stop: method.mutation({ permission: "record", isInternal: true }),
}));

/** A running instance serving an installation, with the releases its deployment serves. */
export interface Endpoint {
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
    /** The space the installation serves. */
    readonly scope: Identifier<"space">;
    /** The deployment it runs. */
    readonly deploymentId: Identifier<"deployment">;
    /** The package-local name of the workload it runs. */
    readonly workload: string;
    /** The server runtime it runs on. */
    readonly runtime: ServerRuntime;
    /** The package release the deployment runs. */
    readonly release: Version;
    /** The oldest caller release it serves, every earlier one when absent. */
    readonly since?: Version;
}

/** A persisted instance observation. */
export type Instance = Select<typeof instance.table>;

/** Instance queries. */
export const Instance = {
    /** List the running instances serving an installation, with their deployments' details. */
    async endpoints(
        database: DatabaseConnection,
        installationId: Identifier<"installation">,
    ): Promise<Endpoint[]> {
        const running = await database
            .select({
                instanceId: instance.table.id,
                scope: deployment.table.scope,
                deploymentId: deployment.table.id,
                workload: deployment.table.workload,
                runtime: deployment.table.runtime,
                release: deployment.table.release,
                since: deployment.table.since,
            })
            .from(instance.table)
            .innerJoin(deployment.table, eq(deployment.table.id, instance.table.deploymentId))
            .where(
                and(
                    eq(deployment.table.installationId, installationId),
                    eq(instance.table.status, "running"),
                ),
            );

        return running.map(({ since, ...endpoint }) =>
            since === null ? endpoint : { ...endpoint, since },
        );
    },
};
