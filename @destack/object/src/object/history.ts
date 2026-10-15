import { through } from "@destack/access";
import { LogPosition } from "@destack/db";
import { schema } from "@destack/schema";
import { field, type FieldsByColumn } from "../field/field.ts";
import type { MethodBuilder } from "../method/record.ts";
import type { ObjectTable } from "./table.ts";
import type { ObjectScope } from "./object.ts";

/** The fields of an activity: one caller's session of changes to an object. */
const activityFields = {
    /** The principal whose changes the session keeps. */
    caller: field.subject().caller(),
    /** When the session's first change committed, in UTC epoch milliseconds. */
    startedAt: field.time(),
    /** When its latest change committed, in UTC epoch milliseconds. */
    endedAt: field.time(),
    /** The log position before the session. */
    from: field.json(LogPosition),
    /** The fields the session changed. */
    fields: field.json(schema.array(schema.string())),
    /** How many changes the session keeps. */
    changes: field.integer(),
};

/** The table of activities, nested under the objects whose changes they keep. */
export type ActivityTable = ObjectTable<
    "activity",
    ObjectScope,
    FieldsByColumn<typeof activityFields>,
    { readonly nested: { readonly in: "any"; readonly receive: "activity" } }
>;

/** A scope type's activities, one per caller's session of changes. */
export const activity = {
    /** Define a scope type's activities, nested under the objects whose changes they keep. */
    definition<const Scope extends ObjectScope>(scope: Scope) {
        return {
            name: "activity",
            plural: "activities",
            scope,
            nested: { in: "any", receive: "activity" },
            fields: activityFields,
            permissions: { read: through("parent", "read") },
            constraints: {
                session: {
                    kind: "index",
                    on: ["parentPackageId", "parentType", "parentId", "callerId", "endedAt"],
                },
            },
        } as const;
    },
};

/** A scope type's checkpoints: named points in an object's history. */
export const checkpoint = {
    /** Define a scope type's checkpoints, nested under the objects whose history they mark. */
    definition<const Scope extends ObjectScope>(scope: Scope) {
        return {
            name: "checkpoint",
            plural: "checkpoints",
            scope,
            nested: { in: "any", receive: "checkpoint" },
            fields: {
                /** The log position the checkpoint marks. */
                position: field.json(LogPosition),
                /** The checkpoint's name. */
                title: field.string(schema.string().min(1).max(200)),
                /** What the checkpoint marks. */
                description: field.string(schema.string().max(2000)).optional(),
            },
            permissions: {
                read: through("parent", "read"),
                checkpoint: through("parent", "checkpoint"),
            },
        } as const;
    },

    /** Declare the checkpoints' methods, a creation marking the transaction's base position. */
    methods: <Definition extends ObjectTable>(method: MethodBuilder<Definition>) => ({
        create: method
            .create("checkpoint", { fields: ["title", "description"], isPredicted: false })
            .handle(async (context, next) =>
                next(context.with({ input: { ...context.input, position: context.base } })),
            ),
        update: method.update("checkpoint", { fields: ["title", "description"] }),
        delete: method.delete("checkpoint"),
    }),
};
