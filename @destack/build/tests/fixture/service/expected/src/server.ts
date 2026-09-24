import { defineDatabase } from "@destack/db/declare";
import { defineSecret, defineVault } from "@destack/vault";
import { defineSchedule } from "@destack/service/schedule";
import { defineService, defineProcedure, defineServiceConnection } from "@destack/service";
import { implement, type ServiceImplementation } from "@destack/service/server";
import type { Schedule, ScheduleImplementation } from "@destack/service/schedule";
import { defineWorkload } from "@destack/service/workload";
import { schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { defineAuditAction } from "@destack/audit";

/** Record a published note under its declaring package. */
export const publishNote = defineAuditAction({
    name: "Note.publish",
    version: 1,
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
    spec: { dialect: "sqlite" },
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
export const notes = defineServiceConnection("notes", service);

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

        return {
            services: [implementService()],
            schedules: [reminders, refresh, appointment].map(implementSchedule),
        };
    },
});

/** The daily reminder schedule. */
export const reminders = defineSchedule({
    name: "reminders",
    timing: "cron",
    cron: "0 9 * * *",
    timezone: "UTC",
    concurrency: "forbid",
    deadline: 60000,
});

/** Repeat from a fixed first occurrence. */
export const refresh = defineSchedule({
    name: "refresh",
    timing: "interval",
    interval: 300000,
    startsAt: 1800000000000,
    endsAt: 1800086400000,
    concurrency: "forbid",
    deadline: 60000,
});

/** Send a reminder at one specified time. */
export const appointment = defineSchedule({
    name: "appointment",
    timing: "once",
    startsAt: 1800000000000,
    concurrency: "allow",
    deadline: 60000,
});

/** Log each occurrence of a reminder schedule. */
function implementSchedule(schedule: Schedule): ScheduleImplementation {
    return {
        schedule,
        run: async (signal) => {
            signal.throwIfAborted();
            instruments.logger.emit({ body: `Reminder ${schedule.name}` });
        },
    };
}
