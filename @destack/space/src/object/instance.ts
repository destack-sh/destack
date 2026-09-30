import {
    and,
    check,
    eq,
    foreignKey,
    index,
    sql,
    type DatabaseConnection,
    type Select,
} from "@destack/db";
import { ConditionMap, defineObject, field, method, type ObjectType } from "@destack/object";
import { identifier, type Identifier } from "@destack/schema";
import type { Endpoint } from "@destack/host/router";
import { deployment } from "./deployment.ts";
import { space } from "./space.ts";

/** Record an instance stopped, as its cell's instance controller decides. */
const stop = method({ permission: null, isSystem: true });

/** An observed running occurrence of a deployment on one host, restarted in place after failures. */
export const instance = defineObject({
    name: "instance",
    plural: "instances",
    scope: space,
    fields: {
        /** The immutable deployment being executed. */
        deploymentId: field.reference<"deployment">((): ObjectType => deployment),
        /** The host administering this instance. */
        hostId: field.string(identifier("host")),
        /** The latest observed execution status. */
        status: field.enum(["starting", "running", "stopped", "failed"]),
        /** Host observations and failure details. */
        conditions: field.json(ConditionMap).default({}),
        /** The last authenticated observation in UTC epoch milliseconds. */
        observedAt: field.time(),
        /** The execution start time in UTC epoch milliseconds. */
        startedAt: field.time().optional(),
        /** The execution finish time in UTC epoch milliseconds, and of the last failure while failed. */
        stoppedAt: field.time().optional(),
        /** The restarts after failures since the instance was recorded. */
        restarts: field.integer().default(0),
    },
    constraints: (entry) => [
        foreignKey({
            columns: [entry.scope, entry.deploymentId],
            foreignColumns: [deployment.table.scope, deployment.table.id],
        }).onDelete("restrict"),
        index("instance_deployment_status").on(entry.deploymentId, entry.status),
        index("instance_host_status").on(entry.hostId, entry.status),
        check(
            "instance_times",
            sql`${entry.stoppedAt} IS NULL OR ${entry.startedAt} IS NULL OR ${entry.stoppedAt} >= ${entry.startedAt}`,
        ),
    ],
    permissions: ["read"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
        stop,
    },
});

/** A persisted instance observation. */
export type Instance = Select<typeof instance.table>;

/** Instance queries. */
export const Instance = {
    /** List the running instances serving an installation, with their deployments' runtimes and releases. */
    async endpoints(
        database: DatabaseConnection,
        installationId: Identifier<"installation">,
    ): Promise<Endpoint[]> {
        const running = await database
            .select({
                instanceId: instance.table.id,
                scope: deployment.table.scope,
                deploymentId: deployment.table.id,
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
