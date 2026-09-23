import { identifier, integer, json, table, text } from "@destack/db";
import { Conditions } from "@destack/model";
import { OperationError } from "@destack/service/operation";
import { deployment } from "./deployment.ts";

/** Observations for individual execution lifetimes, including completed processes. */
export const instance = table("instance", {
    /** Unique identity for this execution lifetime. */
    id: identifier("id", "instance").primaryKey(),
    /** Accepted deployment whose instructions started this execution. */
    deploymentId: identifier("deployment_id", "deployment")
        .notNull()
        .references(() => deployment.id, { onDelete: "restrict" }),
    /** Authorization epoch captured at startup. */
    hostEpoch: integer("host_epoch").notNull(),
    /** Last observed execution status. */
    status: text("status", {
        enum: ["starting", "running", "draining", "stopped", "failed"],
    }).notNull(),
    /** Last observation, in UTC milliseconds. */
    observedAt: integer("observed_at").notNull(),
    /** Execution start, in UTC milliseconds. */
    startedAt: integer("started_at"),
    /** Execution completion, in UTC milliseconds. */
    stoppedAt: integer("stopped_at"),
    /** Public execution failure. */
    error: json("error", OperationError),
    /** Readiness and health observations. */
    conditions: json("conditions", Conditions).notNull(),
});
