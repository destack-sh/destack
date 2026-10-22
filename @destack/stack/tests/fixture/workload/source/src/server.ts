import { defineDatabase } from "@destack/db";
import { defineSecret, defineVault } from "@destack/vault";
import { defineWorkflow } from "@destack/workflow";
import { defineAdapter, defineInstallationBinding, definePackage } from "@destack/object";
import type { PackageHandler } from "@destack/object/server";
import { defineWorkload } from "@destack/object/workload";
import { schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { defineAuditAction } from "@destack/object";

/** Record a published note under its declaring package. */
export const notePublish = defineAuditAction({
    name: "note.publish",
    target: schema.object({ type: schema.literal("note"), id: schema.string() }),
    details: schema.object({ revision: schema.number().int() }),
});

/** Package instruments initialized from build-injected metadata. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The shared application database. */
export const database = defineDatabase({
    name: "main",
    tables: [],
});
/** The application's secret collection. */
export const vault = defineVault({ name: "credentials", spec: {} });
/** The secret selected during installation. */
export const token = defineSecret({ name: "mail-token" });

/** The notes package: its feed adapter and its mail secret. */
export const notes = definePackage({
    adapters: { feed: defineAdapter("feed", { path: "/feed" }) },
    secrets: { "mail-token": token },
});
/** A dependency on an installation of the notes package. */
export const binding = defineInstallationBinding("notes", notes);

/** Implement the notes package, its feed answering the release its modules carry. */
export function implementNotes(): PackageHandler {
    return {
        package: notes,
        router: {},
        fetch: async (request) =>
            new URL(request.url).pathname === "/feed"
                ? Response.json({ path: "/feed", version: notes.version })
                : undefined,
    };
}

/** The web workload serving the notes package. */
export const web = defineWorkload({
    name: "web",
    compute: { cpuTime: 1000 },
    start: async () => {
        instruments.logger.emit({ body: "Workload started" });

        return { packages: [implementNotes()] };
    },
});

/** Send the reminders that are due: daily, every five minutes for a day, and once. */
export const remind = defineWorkflow({
    name: "remind",
    input: schema.object({ at: schema.number() }),
    triggers: {
        reminders: {
            kind: "schedule",
            schedule: {
                timing: { timing: "cron", cron: "0 9 * * *", timezone: "UTC" },
                concurrency: "forbid",
                deadline: 60000,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
        refresh: {
            kind: "schedule",
            schedule: {
                timing: {
                    timing: "interval",
                    interval: 300000,
                    startsAt: 1800000000000,
                    endsAt: 1800086400000,
                },
                concurrency: "forbid",
                deadline: 60000,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
        appointment: {
            kind: "schedule",
            schedule: {
                timing: { timing: "once", startsAt: 1800000000000 },
                concurrency: "allow",
                deadline: 60000,
            },
            input: (occurrence) => ({ at: occurrence.at }),
        },
    },
});
