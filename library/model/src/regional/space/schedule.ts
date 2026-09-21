import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { schema } from "@destack/schema";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";
import { installation } from "./installation.ts";
import { serviceAccount } from "../access/service.ts";

/** A persisted schedule invoking an installed workload operation. */
export const schedule = table(
    "schedule",
    {
        ...recordColumns("schedule"),
        ...provenanceColumns(),
        /** The space containing the installation and execution identity. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The space-local schedule name. */
        name: text("name").notNull(),
        /** The installation serving the operation. */
        installationId: identifier("installation_id", "installation").notNull(),
        /** The workload exporting the operation. */
        workload: text("workload").notNull(),
        /** The exported operation name. */
        operation: text("operation").notNull(),
        /** Arguments validated against the installed operation's schema. */
        arguments: json("arguments", schema.json()).notNull(),
        /** The service account whose current permissions authorise each occurrence. */
        serviceAccountId: identifier("service_account_id", "service-account").notNull(),
        /** The timing form. */
        timing: text("timing", { enum: ["cron", "interval", "once"] }).notNull(),
        /** A five-field cron expression for calendar schedules. */
        cron: text("cron"),
        /** The IANA time zone for calendar schedules. */
        timezone: text("timezone"),
        /** The interval in milliseconds. */
        interval: integer("interval"),
        /** The first interval occurrence or the single occurrence time. */
        startsAt: integer("starts_at"),
        /** The exclusive end time for repeated schedules. */
        endsAt: integer("ends_at"),
        /** Whether to allow, skip, or replace overlapping occurrences. */
        concurrency: text("concurrency", { enum: ["allow", "forbid", "replace"] }).notNull(),
        /** The maximum start delay in milliseconds; zero skips missed occurrences. */
        deadline: integer("deadline").notNull(),
        /** User-controlled pause time, independent of source configuration. */
        pausedAt: integer("paused_at"),
        /** Incremented when timing, arguments, or the target changes. */
        generation: integer("generation").notNull().default(1),
    },
    (entry) => [
        ...provenanceChecks("schedule", entry),
        unique("schedule_space_name").on(entry.spaceId, entry.name),
        foreignKey({
            columns: [entry.spaceId, entry.installationId],
            foreignColumns: [installation.spaceId, installation.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.spaceId, entry.serviceAccountId],
            foreignColumns: [serviceAccount.spaceId, serviceAccount.id],
        }).onDelete("restrict"),
        check(
            "schedule_timing",
            sql`(${entry.timing} = 'cron' AND ${entry.cron} IS NOT NULL AND ${entry.timezone} IS NOT NULL AND ${entry.interval} IS NULL) OR (${entry.timing} = 'interval' AND ${entry.interval} > 0 AND ${entry.interval} IS NOT NULL AND ${entry.startsAt} IS NOT NULL AND ${entry.cron} IS NULL AND ${entry.timezone} IS NULL) OR (${entry.timing} = 'once' AND ${entry.startsAt} IS NOT NULL AND ${entry.endsAt} IS NULL AND ${entry.cron} IS NULL AND ${entry.timezone} IS NULL AND ${entry.interval} IS NULL)`,
        ),
        check("schedule_concurrency", sql`${entry.concurrency} IN ('allow', 'forbid', 'replace')`),
        check("schedule_deadline", sql`${entry.deadline} >= 0`),
        check("schedule_generation", sql`${entry.generation} > 0`),
        check(
            "schedule_end",
            sql`${entry.endsAt} IS NULL OR ${entry.startsAt} IS NULL OR ${entry.endsAt} > ${entry.startsAt}`,
        ),
        check(
            "schedule_names",
            sql`length(${entry.name}) > 0 AND length(${entry.workload}) > 0 AND length(${entry.operation}) > 0`,
        ),
        index("schedule_installation").on(entry.installationId),
    ],
);
/** A persisted schedule, declared in source or created through the API. */
export type Schedule = Select<typeof schedule>;
