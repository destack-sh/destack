import { schema } from "@destack/schema";
import { AuditActionDescription, describeAuditAction } from "@destack/audit/inspect";
import type { AuditAction } from "@destack/audit";
import { BucketDeclaration } from "@destack/bucket/inspect";
import { DatabaseDeclaration } from "@destack/db/declare";
import { DatabaseSchemaDescription, describeSchema } from "@destack/db/inspect";
import type { DatabaseSchema } from "@destack/db";
import { SecretDeclaration, VaultDeclaration } from "@destack/vault/inspect";
import { ScheduleDeclaration } from "@destack/service/schedule";
import {
    inspectService,
    ServiceInspection,
    ServiceConnectionDeclaration,
    describeServiceConnection,
} from "@destack/service/inspect";
import type { ServiceDefinition, ServiceConnection } from "@destack/service";
import type { Package } from "@destack/package";
import { SpaceDefinition } from "@destack/space";
import { AccountDefinition } from "@destack/model/declare";
import { AccessDeclaration } from "@destack/access/inspect";
import type { ObjectType } from "@destack/access/declare";
import { CronExpressionParser } from "cron-parser";
import { BuildError } from "../error/index.ts";
import {
    SettingDescription,
    describeSetting,
    SettingAssignmentDefinition,
    SettingPolicyDefinition,
} from "@destack/setting/inspect";
import type { Setting } from "@destack/setting";

/** Declaration constructors and their serialized descriptions. */
export const INSPECTORS = {
    defineServiceConnection: {
        package: "@destack/service",
        kind: "service-connection",
        schema: ServiceConnectionDeclaration,
    },
    defineSetting: { package: "@destack/setting", kind: "setting", schema: SettingDescription },
    defineSettingAssignment: {
        package: "@destack/setting",
        kind: "setting-assignment",
        schema: SettingAssignmentDefinition,
    },
    defineSettingPolicy: {
        package: "@destack/setting",
        kind: "setting-policy",
        schema: SettingPolicyDefinition,
    },
    defineAuditAction: {
        package: "@destack/audit",
        kind: "audit-action",
        schema: AuditActionDescription,
    },
    defineDatabaseSchema: {
        package: "@destack/db",
        kind: "database-schema",
        schema: schema.object({
            name: DatabaseSchemaDescription.shape.name,
            sqlite: DatabaseSchemaDescription,
            postgresql: DatabaseSchemaDescription,
        }),
    },
    defineSchedule: { package: "@destack/service", kind: "schedule", schema: ScheduleDeclaration },
    defineBucket: { package: "@destack/bucket", kind: "resource", schema: BucketDeclaration },
    defineDatabase: { package: "@destack/db", kind: "resource", schema: DatabaseDeclaration },
    defineVault: { package: "@destack/vault", kind: "resource", schema: VaultDeclaration },
    defineSecret: { package: "@destack/vault", kind: "secret", schema: SecretDeclaration },
    defineService: { package: "@destack/service", kind: "service", schema: ServiceInspection },
    defineSpace: { package: "@destack/space", kind: "space", schema: SpaceDefinition },
    defineAccount: { package: "@destack/model", kind: "account", schema: AccountDefinition },
    defineObject: { package: "@destack/access", kind: "access", schema: AccessDeclaration },
} as const;

/** A supported declaration constructor. */
export type InspectorName = keyof typeof INSPECTORS;

/** Describe an evaluated declaration with its domain library. */
export async function inspectDeclaration(
    name: InspectorName,
    value: unknown,
    owner: Package,
): Promise<Record<string, unknown>> {
    // qualify connection dependencies by their actual declaring package
    if (name === "defineServiceConnection") {
        const declaration = describeServiceConnection(value as ServiceConnection);
        if (declaration.packageId !== owner.id) {
            throw new BuildError(
                "INSPECTION_FAILED",
                "service connection declares a different package",
            );
        }

        return declaration;
    }

    // retain the native declaration's schema and verify its immutable package identity
    if (name === "defineSetting") {
        const description = describeSetting(value as Setting);
        if (
            description.package.id !== owner.id ||
            description.package.version !== owner.version ||
            description.package.name !== owner.name
        ) {
            throw new BuildError("INSPECTION_FAILED", "setting declares a different package");
        }

        return description;
    }
    // retain qualified access declarations without evaluating application records
    if (name === "defineObject") {
        const declaration = AccessDeclaration.parse((value as ObjectType).definition);
        if (declaration.packageId !== owner.id) {
            throw new BuildError(
                "INSPECTION_FAILED",
                "access declaration belongs to a different package",
            );
        }

        return declaration;
    }

    // retain the declaring package even when another package emits the action
    if (name === "defineAuditAction") {
        const description = describeAuditAction(value as AuditAction);
        if (
            description.package.id !== owner.id ||
            description.package.name !== owner.name ||
            description.package.version !== owner.version
        ) {
            throw new BuildError("INSPECTION_FAILED", "audit action declares a different package");
        }

        return description;
    }
    // describe both SQL dialects without opening a database
    if (name === "defineDatabaseSchema") {
        const database = value as DatabaseSchema;

        return INSPECTORS[name].schema.parse({
            name: database.name,
            sqlite: describeSchema(database, "sqlite"),
            postgresql: describeSchema(database, "postgresql"),
        });
    }

    // describe procedures through the service library using the declaring package version
    if (name === "defineService") {
        const service = value as ServiceDefinition;

        return inspectService(service);
    }

    // validate schedule expressions alongside their declared timing
    if (name === "defineSchedule") {
        const schedule = ScheduleDeclaration.parse(value);
        if (schedule.timing === "cron") {
            new Intl.DateTimeFormat("en", { timeZone: schedule.timezone });
            CronExpressionParser.parse(schedule.cron, { tz: schedule.timezone });
        }
        if (
            "endsAt" in schedule &&
            schedule.endsAt !== undefined &&
            schedule.startsAt !== undefined &&
            schedule.endsAt <= schedule.startsAt
        ) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Schedule ends before its first occurrence: ${schedule.name}`,
            );
        }
    }

    return INSPECTORS[name].schema.parse(value);
}
