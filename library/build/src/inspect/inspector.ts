import { schema } from "@destack/schema";
import { BucketDeclaration } from "@destack/storage/inspect";
import { DatabaseDeclaration } from "@destack/db/declare";
import { DatabaseSchemaDescription, describeSchema } from "@destack/db/inspect";
import type { DatabaseSchema } from "@destack/db";
import { SecretDeclaration, VaultDeclaration } from "@destack/vault/inspect";
import { ScheduleDeclaration } from "@destack/service/schedule";
import { inspectService, ServiceInspection } from "@destack/service/inspect";
import type { ServiceDefinition } from "@destack/service";
import type { Package } from "@destack/package";
import { SpaceDefinition } from "@destack/space";
import { CronExpressionParser } from "cron-parser";
import { BuildError } from "../error/index.ts";

/** Declaration constructors and their serialized descriptions. */
export const INSPECTORS = {
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
    defineBucket: { package: "@destack/storage", kind: "resource", schema: BucketDeclaration },
    defineDatabase: { package: "@destack/db", kind: "resource", schema: DatabaseDeclaration },
    defineVault: { package: "@destack/vault", kind: "resource", schema: VaultDeclaration },
    defineSecret: { package: "@destack/vault", kind: "secret", schema: SecretDeclaration },
    defineService: { package: "@destack/service", kind: "service", schema: ServiceInspection },
    defineSpace: { package: "@destack/space", kind: "space", schema: SpaceDefinition },
} as const;

/** A supported declaration constructor. */
export type InspectorName = keyof typeof INSPECTORS;

/** Describe an evaluated declaration with its domain library. */
export async function inspectDeclaration(
    name: InspectorName,
    value: unknown,
    owner: Package,
): Promise<Record<string, unknown>> {
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

        return inspectService(service, { info: { title: service.name, version: owner.version } });
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
