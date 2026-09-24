import { defineAuditAction } from "@destack/audit";
import { schema } from "@destack/schema";
import { SettingReference } from "../setting/setting.ts";

/** Record a settings edit without including its potentially private value. */
export const changeAssignment = defineAuditAction({
    name: "Assignment.update",
    version: 1,
    targets: schema.object({
        assignment: schema.object({
            type: schema.literal("setting-assignment"),
            id: schema.string(),
        }),
    }),
    details: schema.object({
        setting: SettingReference,
        revision: schema.uuid(),
        operation: schema.enum(["set", "reset", "detach"]),
    }),
});

/** Record policy lifecycle changes without storing policy values in audit details. */
export const changePolicy = defineAuditAction({
    name: "Policy.update",
    version: 1,
    targets: schema.object({
        policy: schema.object({ type: schema.literal("setting-policy"), id: schema.string() }),
    }),
    details: schema.object({
        setting: SettingReference,
        revision: schema.uuid(),
        operation: schema.enum(["set", "remove", "detach"]),
    }),
});
