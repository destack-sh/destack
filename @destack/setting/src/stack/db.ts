import {
    defineDatabaseSchema,
    table,
    text,
    json,
    integer,
    index,
    uniqueIndex,
    check,
    sql,
} from "@destack/db";
import { RecordProvenance } from "@destack/model/source";
import { schema } from "@destack/schema";
import { defineRequestTable } from "@destack/service/database";
import { auditOutboxSchema } from "@destack/audit/outbox";
import { SettingAssignment } from "../setting/assignment.ts";
import { SettingPolicy } from "../setting/policy.ts";

/** Explicit assignments at one exact setting target. */
export const settingAssignment = table(
    "setting_assignment",
    {
        /** Stable assignment identity. */
        id: text("id").primaryKey().notNull(),
        /** Declaring package. */
        packageId: text("package_id").notNull(),
        /** Personal, shared space or host configuration. */
        scope: text("scope", { enum: ["user", "space", "host"] }).notNull(),
        /** Authority issuing the user identity. */
        userAuthority: text("user_authority"),
        /** Personal recipient. */
        userId: text("user_id"),
        /** Shared space or personal space refinement. */
        spaceId: text("space_id"),
        /** Host receiving local configuration. */
        hostId: text("host_id"),
        /** Optional consuming app refinement. */
        consumerPackageId: text("consumer_package_id"),
        /** Optional installation refinement. */
        installationId: text("installation_id"),
        /** Optional device refinement. */
        deviceId: text("device_id"),
        /** Stable hierarchical declaration name. */
        name: text("name").notNull(),
        /** Complete assigned value. */
        value: json("value", SettingAssignment.shape.value).notNull(),
        /** Revision used for conditional writes. */
        revision: text("revision").notNull(),
        /** Last applied source declaration. */
        provenance: json("provenance", RecordProvenance),
        /** Canonical declaring account, space or installation. */
        source: text("source"),
        /** Source detachment time in UTC milliseconds. */
        detachedAt: integer("detached_at"),
        /** Creation time in UTC milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last mutation time in UTC milliseconds. */
        updatedAt: integer("updated_at").notNull(),
    },
    (assignment) => [
        index("setting_assignment_user").on(
            assignment.userAuthority,
            assignment.userId,
            assignment.packageId,
            assignment.name,
        ),
        index("setting_assignment_space").on(
            assignment.spaceId,
            assignment.packageId,
            assignment.name,
        ),
        index("setting_assignment_host").on(
            assignment.hostId,
            assignment.packageId,
            assignment.name,
        ),
        uniqueIndex("setting_assignment_target").on(
            assignment.packageId,
            assignment.name,
            assignment.scope,
            sql`coalesce(${assignment.userAuthority}, '')`,
            sql`coalesce(${assignment.userId}, '')`,
            sql`coalesce(${assignment.spaceId}, '')`,
            sql`coalesce(${assignment.hostId}, '')`,
            sql`coalesce(${assignment.consumerPackageId}, '')`,
            sql`coalesce(${assignment.installationId}, '')`,
            sql`coalesce(${assignment.deviceId}, '')`,
        ),
        index("setting_assignment_source").on(assignment.source),
        check(
            "setting_assignment_scope",
            sql`(
            (${assignment.scope} = 'user' AND ${assignment.userAuthority} IS NOT NULL AND ${assignment.userId} IS NOT NULL AND ${assignment.hostId} IS NULL)
            OR (${assignment.scope} = 'space' AND ${assignment.spaceId} IS NOT NULL AND ${assignment.userAuthority} IS NULL AND ${assignment.userId} IS NULL AND ${assignment.hostId} IS NULL AND ${assignment.consumerPackageId} IS NULL AND ${assignment.deviceId} IS NULL)
            OR (${assignment.scope} = 'host' AND ${assignment.hostId} IS NOT NULL AND ${assignment.userAuthority} IS NULL AND ${assignment.userId} IS NULL AND ${assignment.spaceId} IS NULL AND ${assignment.installationId} IS NULL AND ${assignment.consumerPackageId} IS NULL AND ${assignment.deviceId} IS NULL)
        ) AND (${assignment.installationId} IS NULL OR ${assignment.spaceId} IS NOT NULL)`,
        ),
    ],
);

/** Policies retained under the issuing administrative authority. */
export const settingPolicy = table(
    "setting_policy",
    {
        /** Stable policy identifier. */
        id: text("id").primaryKey().notNull(),
        /** Kind of administering authority. */
        authority: text("authority", { enum: ["account", "space", "host"] }).notNull(),
        /** The administering account. */
        accountId: text("account_id"),
        /** The administering space. */
        spaceId: text("space_id"),
        /** The administering host. */
        hostId: text("host_id"),
        /** The governed declaration. */
        setting: json("setting", SettingPolicy.shape.setting).notNull(),
        /** Optional installation refinement. */
        installationId: text("installation_id"),
        /** Recommended or required enforcement. */
        mode: text("mode", { enum: ["recommended", "required"] }).notNull(),
        /** Complete policy value. */
        value: json("value", schema.json()).notNull(),
        /** Revision used for conditional writes. */
        revision: text("revision").notNull(),
        /** Last applied source declaration. */
        provenance: json("provenance", RecordProvenance),
        /** Canonical declaring account, space or installation. */
        source: text("source"),
        /** Source detachment time in UTC milliseconds. */
        detachedAt: integer("detached_at"),
        /** Creation time in UTC milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last mutation time in UTC milliseconds. */
        updatedAt: integer("updated_at").notNull(),
    },
    (policy) => [
        index("setting_policy_account").on(policy.accountId, policy.id),
        index("setting_policy_space").on(policy.spaceId, policy.id),
        index("setting_policy_host").on(policy.hostId, policy.id),
        index("setting_policy_source").on(policy.source),
        check(
            "setting_policy_authority",
            sql`
            (${policy.authority} = 'account' AND ${policy.accountId} IS NOT NULL AND ${policy.spaceId} IS NULL AND ${policy.hostId} IS NULL)
            OR (${policy.authority} = 'space' AND ${policy.spaceId} IS NOT NULL AND ${policy.accountId} IS NULL AND ${policy.hostId} IS NULL)
            OR (${policy.authority} = 'host' AND ${policy.hostId} IS NOT NULL AND ${policy.accountId} IS NULL AND ${policy.spaceId} IS NULL)`,
        ),
    ],
);

/** Mutation replay records committed with the assignment and audit event. */
export const settingRequest = defineRequestTable("setting_request");

/** Conditional application of complete account, space or installation settings. */
export const settingSource = table("setting_source", {
    /** Canonical declaring account, space or installation. */
    key: text("key").primaryKey(),
    /** Last accepted reconciliation number. */
    revision: integer("revision").notNull(),
});

/** Settings storage composed into the administering service's database. */
export const settingSchema = defineDatabaseSchema({
    name: "destack-setting",
    tables: {
        settingAssignment,
        settingPolicy,
        settingRequest,
        settingSource,
    },
    dependencies: [auditOutboxSchema],
    migrations: new URL("./migration/", import.meta.url),
});
