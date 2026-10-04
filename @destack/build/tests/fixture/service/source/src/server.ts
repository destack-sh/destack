import { defineDatabase } from "@destack/db";
import { defineSecret, defineVault } from "@destack/vault";
import { defineTrigger } from "@destack/service/trigger";
import { defineService, defineProcedure, defineServiceBinding } from "@destack/service";
import { implement, type ServiceImplementation } from "@destack/service/server";
import { defineWorkload } from "@destack/service/workload";
import { schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { defineAuditAction } from "@destack/audit";

/** Record a published note under its declaring package. */
export const publishNote = defineAuditAction({
    name: "note.publish",
    targets: schema.object({
        note: schema.object({ type: schema.literal("note"), id: schema.string() }),
    }),
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
/** The public notes API. */
export const router = {
    list: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(schema.object({ path: schema.string() })),
};
/** The public HTTP service. */
export const service = defineService("notes", router);
/** A dependency on the installation's notes service. */
export const notes = defineServiceBinding("notes", service);

/** Implement the public notes procedures. */
export function implementService(): ServiceImplementation {
    const implementation = implement(router);

    return {
        service,
        router: implementation.router({
            list: implementation.list.handler(() => ({ path: "/notes" })),
        }),
        authorize: async () => {},
    };
}

/** The web workload hosting notes and reminders. */
export const web = defineWorkload({
    name: "web",
    compute: { cpuTime: 1000 },
    start: async () => {
        instruments.logger.emit({ body: "Workload started" });

        return { services: [implementService()] };
    },
});

/** Send the reminders that are due. */
const remind = { method: "reminder.send", input: {}, release: "2026.9.0" };

/** The daily reminder schedule. */
export const reminders = defineTrigger({
    name: "reminders",
    on: {
        schedule: {
            timing: { timing: "cron", cron: "0 9 * * *", timezone: "UTC" },
            concurrency: "forbid",
            deadline: 60000,
            call: remind,
        },
    },
});

/** Repeat from a fixed first occurrence. */
export const refresh = defineTrigger({
    name: "refresh",
    on: {
        schedule: {
            timing: {
                timing: "interval",
                interval: 300000,
                startsAt: 1800000000000,
                endsAt: 1800086400000,
            },
            concurrency: "forbid",
            deadline: 60000,
            call: remind,
        },
    },
});

/** Send a reminder at one specified time. */
export const appointment = defineTrigger({
    name: "appointment",
    on: {
        schedule: {
            timing: { timing: "once", startsAt: 1800000000000 },
            concurrency: "allow",
            deadline: 60000,
            call: remind,
        },
    },
});
