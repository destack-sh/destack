import { none, through, union } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { Instant, schema } from "@destack/schema";
import { TriggerOn } from "@destack/runtime/trigger";
import { installation } from "./installation.ts";
import { space } from "@destack/account/object";

/** A trigger of an installation: what fires it, declared by its package or created by it. */
export const trigger = defineObject(
    {
        name: "trigger",
        plural: "triggers",
        scope: space,
        fields: {
            /** The installation whose calls run. */
            installation: field.reference(installation, { delete: "cascade" }),
            /** Whether the package declares the trigger, or the installation created it. */
            origin: field.enum(["declared", "created"]).default("created"),
            /** The trigger's name within its installation. */
            name: field.string(DeclarationName),

            // what fires and what runs
            /** What fires the trigger. */
            on: field.json(TriggerOn),
            /** Whether the trigger pauses. */
            isPaused: field.boolean().default(false),

            // what fired
            /** The latest occurrence a schedule recorded a run of, in UTC epoch milliseconds. */
            lastScheduledAt: field.time().optional(),
        },
        permissions: {
            read: through("installation", "read"),
            write: through("installation", "run"),
            pause: union(through("installation", "run"), through("installation", "update")),
            declare: none(),
            fire: none(),
        },
        reserved: ["write"],
        constraints: {
            // keep declared trigger names apart from the ones the installation created
            name: { kind: "index", on: ["installationId", "origin", "name"], unique: true },
        },
    },
    (method) => ({
        create: method.create("write", { fields: ["installationId", "name", "on"] }),
        update: method.update("write", { fields: ["on"] }),

        /** Pause the trigger, which fires nothing until resumed. */
        pause: method.mutation({ permission: "pause", inverse: "resume" }),

        /** Resume a paused trigger. */
        resume: method.mutation({ permission: "pause", inverse: "pause" }),

        declare: method.create("declare", { isInternal: true }),
        redeclare: method.update("declare", { isInternal: true }),
        retire: method.delete("declare", { isInternal: true }),
        fire: method.mutation({
            permission: "fire",
            isInternal: true,
            input: schema.object({
                /** The time the occurrences are due by, in UTC epoch milliseconds. */
                at: Instant,
            }),
        }),
    }),
).implement({
    methods: {
        pause: (call) => call.update({ isPaused: true }),
        resume: (call) => call.update({ isPaused: false }),
    },
});

/** A trigger of an installation. */
export type Trigger = Select<typeof trigger.table>;
