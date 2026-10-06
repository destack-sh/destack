import { AuditCall } from "./call.ts";
import { identifier, integer, text, json, defineTable, index, uniqueIndex } from "@destack/db";

/** The audited calls of each scope. */
export const auditCall = defineTable(
    "call",
    {
        /** The call's identity. */
        id: identifier("id", "call").primaryKey(),
        /** The scope whose history keeps the call. */
        scope: text("scope").notNull(),
        /** The object type and method, such as page.create. */
        method: text("method").notNull(),
        /** The package serving the call. */
        packageId: identifier("package_id", "package").notNull(),
        /** The actor's key. */
        actor: text("actor").notNull(),
        /** A write, a read of data, or a refused call. */
        category: text("category", { enum: ["activity", "access", "denial"] }).notNull(),
        /** How the call ended, absent while it runs. */
        outcome: text("outcome", { enum: ["success", "failure", "denied", "cancelled"] }),
        /** The start time, in UTC milliseconds. */
        startedAt: integer("started_at").notNull(),
        /** The acceptance time, in UTC milliseconds. */
        recordedAt: integer("recorded_at").notNull(),
        /** The complete call, without its result value. */
        call: json("call", AuditCall).notNull(),
    },
    {
        log: { retention: "window" },
        constraints: (entry) => [
            index("call_scope_time").on(entry.scope, entry.recordedAt, entry.id),
            index("call_category_time").on(entry.scope, entry.category, entry.recordedAt, entry.id),
            index("call_method_time").on(entry.method, entry.recordedAt, entry.id),
            index("call_package_time").on(entry.packageId, entry.recordedAt, entry.id),
            index("call_actor_time").on(entry.actor, entry.recordedAt, entry.id),
        ],
    },
);

/** The objects audited calls name. */
export const auditTarget = defineTable(
    "target",
    {
        /** The target identity. */
        id: identifier("id", "audit-target").primaryKey(),
        /** The call naming the target. */
        callId: identifier("call_id", "call")
            .notNull()
            .references(() => auditCall.id, { onDelete: "cascade" }),
        /** The scope of the call's history. */
        scope: text("scope").notNull(),
        /** The name of the target within its call. */
        role: text("role").notNull(),
        /** The object's type. */
        type: text("type").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
    },
    {
        log: { retention: "window" },
        constraints: (target) => [
            uniqueIndex("target_role").on(target.callId, target.role),
            index("target_object").on(target.type, target.objectId, target.callId),
        ],
    },
);
