import { describeAuditAction } from "@destack/audit/inspect";
import type { AuditAction } from "@destack/audit";
import { BucketDeclaration } from "@destack/bucket/inspect";
import { DatabaseDeclaration } from "@destack/db/declare";
import { DatabaseSchemaDescription, describeSchema } from "@destack/db/inspect";
import type { DatabaseSchema } from "@destack/db";
import { SecretDeclaration, VaultDeclaration } from "@destack/vault/inspect";
import { ScheduleDeclaration } from "@destack/service/schedule";
import { inspectService, describeServiceConnection } from "@destack/service/inspect";
import type { ServiceDefinition, ServiceConnection } from "@destack/service";
import type { Package } from "@destack/package";
import { SpaceDefinition } from "@destack/space";
import { AccountDefinition } from "@destack/model/declare";
import { AccessDeclaration } from "@destack/access/inspect";
import type { ObjectType } from "@destack/access/declare";
import { CronExpressionParser } from "cron-parser";
import { BuildError } from "../error/index.ts";
import {
    describeSetting,
    SettingAssignmentDefinition,
    SettingPolicyDefinition,
} from "@destack/setting/inspect";
import type { Setting } from "@destack/setting";

/** Declaration constructors and their domain inspectors. */
export const INSPECTORS = {
    defineServiceConnection: {
        package: "@destack/service",
        kind: "service-connection",
        describe(value, owner) {
            const description = describeServiceConnection(value as ServiceConnection);
            requirePackage(
                { id: description.packageId },
                owner,
                "service connection declares a different package",
            );

            return description;
        },
    },
    defineSetting: {
        package: "@destack/setting",
        kind: "setting",
        describe(value, owner) {
            const description = describeSetting(value as Setting);
            requirePackage(description.package, owner, "setting declares a different package");

            return description;
        },
    },
    defineSettingAssignment: {
        package: "@destack/setting",
        kind: "setting-assignment",
        describe: (value) => SettingAssignmentDefinition.parse(value),
    },
    defineSettingPolicy: {
        package: "@destack/setting",
        kind: "setting-policy",
        describe: (value) => SettingPolicyDefinition.parse(value),
    },
    defineAuditAction: {
        package: "@destack/audit",
        kind: "audit-action",
        describe(value, owner) {
            const description = describeAuditAction(value as AuditAction);
            requirePackage(description.package, owner, "audit action declares a different package");

            return description;
        },
    },
    defineDatabaseSchema: {
        package: "@destack/db",
        kind: "database-schema",
        describe(value) {
            const database = value as DatabaseSchema;

            return {
                name: database.name,
                sqlite: DatabaseSchemaDescription.parse(describeSchema(database, "sqlite")),
                postgresql: DatabaseSchemaDescription.parse(describeSchema(database, "postgresql")),
            };
        },
    },
    defineSchedule: {
        package: "@destack/service",
        kind: "schedule",
        describe: (value) => describeSchedule(ScheduleDeclaration.parse(value)),
    },
    defineBucket: {
        package: "@destack/bucket",
        kind: "resource",
        describe: (value) => BucketDeclaration.parse(value),
    },
    defineDatabase: {
        package: "@destack/db",
        kind: "resource",
        describe: (value) => DatabaseDeclaration.parse(value),
    },
    defineVault: {
        package: "@destack/vault",
        kind: "resource",
        describe: (value) => VaultDeclaration.parse(value),
    },
    defineSecret: {
        package: "@destack/vault",
        kind: "secret",
        describe: (value) => SecretDeclaration.parse(value),
    },
    defineService: {
        package: "@destack/service",
        kind: "service",
        describe: (value) => inspectService(value as ServiceDefinition),
    },
    defineSpace: {
        package: "@destack/space",
        kind: "space",
        describe: (value) => SpaceDefinition.parse(value),
    },
    defineAccount: {
        package: "@destack/model",
        kind: "account",
        describe: (value) => AccountDefinition.parse(value),
    },
    defineObject: {
        package: "@destack/access",
        kind: "access",
        describe(value, owner) {
            const description = AccessDeclaration.parse((value as ObjectType).definition);
            requirePackage(
                { id: description.packageId },
                owner,
                "access declaration belongs to a different package",
            );

            return description;
        },
    },
} satisfies Record<string, DeclarationInspector>;

/** A domain constructor and the description it contributes to a package. */
interface DeclarationInspector {
    /** The package that defines the constructor. */
    readonly package: string;
    /** The declaration kind stored in the manifest. */
    readonly kind: string;
    /** Describe the evaluated value under its declaring package. */
    describe(value: unknown, owner: Package): Record<string, unknown>;
}

/** A supported declaration constructor. */
export type InspectorName = keyof typeof INSPECTORS;

/** Require declaration identity to match the inspected source package. */
function requirePackage(
    actual: Pick<Package, "id"> | Package,
    expected: Package,
    message: string,
): void {
    if (
        actual.id !== expected.id ||
        ("name" in actual && (actual.name !== expected.name || actual.version !== expected.version))
    ) {
        throw new BuildError("INSPECTION_FAILED", message);
    }
}

/** Check calendar expressions and occurrence bounds before describing a schedule. */
function describeSchedule(schedule: ScheduleDeclaration): ScheduleDeclaration {
    // resolve calendar expressions against their declared time zone
    if (schedule.timing === "cron") {
        new Intl.DateTimeFormat("en", { timeZone: schedule.timezone });
        CronExpressionParser.parse(schedule.cron, { tz: schedule.timezone });
    }

    // require a nonempty occurrence interval
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

    return schedule;
}
