import { defineEventKind } from "@destack/event/declare";
import { TELEMETRY_ACCESS } from "./access.ts";
import { schema } from "@destack/schema";
import { AttributeKey, Instrumentation, PersonalAttributes } from "./attribute.ts";
import { DAY, TELEMETRY_FLUSH } from "./policy.ts";

/** A log record an installation or a machine emitted, as OpenTelemetry's log data model records it. */
export const log = defineEventKind({
    name: "log",
    description: "A log record an installation or a machine emitted.",
    keys: schema.object({
        /** The installation that emitted it, null for the machine's own records. */
        installation: schema.string().nullable(),
        /** The digest of the emitting build's manifest. */
        build: schema.string().nullable(),
        /** The event name, or a text body naming none. */
        name: schema.string(),
        /** The severity, 1 to 24 as OpenTelemetry numbers it, 0 when unspecified. */
        severity: schema.number().int().min(0).max(24),
        /** The trace it was emitted in, as 32 hexadecimal digits. */
        trace: schema.string().nullable(),
        /** The issue its exception is grouped into. */
        issue: schema.string().nullable(),
        /** The person it acted for, whose key seals its personal attributes. */
        person: schema.string().nullable(),
        /** The attributes, the personal ones left out. */
        attributes: AttributeKey,
    }),
    data: schema.object({
        /** The body, as text or JSON. */
        body: schema.string().exactOptional(),
        /** The span it was emitted in, as 16 hexadecimal digits. */
        span: schema.string().exactOptional(),
        /** The workload instance that emitted it. */
        instance: schema.string().exactOptional(),
        /** The library that recorded it. */
        instrumentation: Instrumentation,
        /** When the collector observed it, in Unix microseconds. */
        observed: schema.number().int().exactOptional(),
        /** The attributes the emitter marked personal. */
        personal: PersonalAttributes.exactOptional(),
    }),
    delivery: "at-most-once",
    policy: { flush: TELEMETRY_FLUSH, retention: 30 * DAY },
    access: TELEMETRY_ACCESS,
    subject: "person",
});

/** The levels of log records, one per band of four OpenTelemetry severity numbers. */
export const LOG_LEVELS = ["trace", "debug", "info", "warn", "error", "fatal"] as const;

/** The level of a log record's severity. */
export type LogLevel = (typeof LOG_LEVELS)[number];

/** The OpenTelemetry severity numbers in each level. */
const SEVERITY_BAND = 4;

/** Read the level of a severity, absent for an unspecified one. */
export function levelOf(severity: number): LogLevel | undefined {
    return severity === 0 ? undefined : LOG_LEVELS[Math.floor((severity - 1) / SEVERITY_BAND)];
}
