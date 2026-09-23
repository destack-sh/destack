import { defineAuditAction } from "@destack/audit";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { SettingReference } from "../setting/setting.ts";
import metadata from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };

/** Record a settings edit without including its potentially private value. */
export const changeAssignment = defineAuditAction({
    package: Package.parse({ id: definition.id, name: metadata.name, version: metadata.version }),
    name: "assignment.update",
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
    package: Package.parse({ id: definition.id, name: metadata.name, version: metadata.version }),
    name: "policy.update",
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
