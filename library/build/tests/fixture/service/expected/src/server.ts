import { defineDatabase } from "@destack/db/declare";
import { defineSecret, defineVault } from "@destack/vault";
import { defineSchedule } from "@destack/service/schedule";
import { defineService, defineProcedure } from "@destack/service";
import { implement, ServiceHandler } from "@destack/service/server";
import { Health } from "@destack/service/health";
import { schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { defineAuditAction } from "@destack/audit";

/** Record a published note under its declaring package. */
export const publishNote = defineAuditAction({
    package: import.meta.destack.package,
    name: "note.publish",
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
export const service = defineService(
    {
        name: "notes",
        version: 1,
        protocol: "http",
        handler: "fetch",
    },
    router,
);

/** The typed notes implementation. */
const implementation = implement(router);
/** Readiness of the initialized HTTP handler. */
const health = new Health("notes");
health.set("serving");
/** The HTTP dispatcher used by both supported runtimes. */
const handler = new ServiceHandler(
    {
        list: implementation.list.handler(() => ({ path: "/notes" })),
    },
    { health },
);

/** Respond through the emitted handler. */
export async function fetch(request: Request): Promise<Response> {
    instruments.logger.emit({ body: "Request received" });

    const result = await handler.handle(request);

    return result.response ?? new Response("Not found", { status: 404 });
}
/** The daily reminder schedule. */
export const reminders = defineSchedule({
    name: "reminders",
    version: 1,
    handler: "remind",
    timing: "cron",
    cron: "0 9 * * *",
    timezone: "UTC",
    concurrency: "forbid",
    deadline: 60000,
});

/** Repeat from a fixed first occurrence. */
export const refresh = defineSchedule({
    name: "refresh",
    version: 1,
    handler: "remind",
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
    version: 1,
    handler: "remind",
    timing: "once",
    startsAt: 1800000000000,
    concurrency: "allow",
    deadline: 60000,
});

/** Return the scheduled occurrence identifier. */
export function remind(occurrence: { id: string }): string {
    return occurrence.id;
}
