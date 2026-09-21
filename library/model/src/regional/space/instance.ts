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
} from "@destack/db";
import { Conditions } from "../../record/index.ts";
import { deployment } from "./deployment.ts";
import { spaceHost } from "./host.ts";
import { defineSchema, schema } from "@destack/schema";

/** Instance lifecycle states shared by schemas and database constraints. */
export const INSTANCE_STATES = ["starting", "running", "draining", "stopped", "failed"] as const;
/** The last observed instance lifecycle state. */
export const InstanceState = defineSchema(schema.enum(INSTANCE_STATES));

/** An observed running occurrence of a deployment. */
export const instance = table(
    "instance",
    {
        ...recordColumns("instance"),
        /** The installation's space. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The immutable deployment being executed. */
        deploymentId: identifier("deployment_id", "deployment").notNull(),
        /** The host administering this instance. */
        hostId: identifier("host_id", "host").notNull(),
        /** The space-host authorization epoch captured when execution started. */
        hostEpoch: integer("host_epoch").notNull(),
        /** The provider's instance reference, when one is available. */
        reference: text("reference"),
        /** The latest observed execution state. */
        state: text("state", {
            enum: INSTANCE_STATES,
        }).notNull(),
        /** Host observations and failure details. */
        conditions: json("conditions", Conditions)
            .notNull()
            .default(sql`'{}'`),
        /** The last authenticated observation in UTC epoch milliseconds. */
        observedAt: integer("observed_at").notNull(),
        /** The expiry of the host's execution lease, when execution uses leases. */
        leaseExpiresAt: integer("lease_expires_at"),
        /** The execution start time in UTC epoch milliseconds. */
        startedAt: integer("started_at"),
        /** The execution finish time in UTC epoch milliseconds. */
        stoppedAt: integer("stopped_at"),
    },
    (entry) => [
        foreignKey({
            columns: [entry.spaceId, entry.deploymentId],
            foreignColumns: [deployment.spaceId, deployment.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.spaceId, entry.hostId],
            foreignColumns: [spaceHost.spaceId, spaceHost.hostId],
        }).onDelete("restrict"),
        index("instance_deployment_state").on(entry.deploymentId, entry.state),
        index("instance_host_state").on(entry.hostId, entry.state),
        check("instance_epoch", sql`${entry.hostEpoch} > 0`),
        check(
            "instance_state",
            sql`${entry.state} IN (${sql.join(
                INSTANCE_STATES.map((value) => sql.raw(`'${value}'`)),
                sql`, `,
            )})`,
        ),
        check(
            "instance_times",
            sql`${entry.stoppedAt} IS NULL OR ${entry.startedAt} IS NULL OR ${entry.stoppedAt} >= ${entry.startedAt}`,
        ),
    ],
);

/** A persisted instance observation. */
export type Instance = Select<typeof instance>;
