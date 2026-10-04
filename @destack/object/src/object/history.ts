import { through } from "@destack/access";
import { index, LogPosition } from "@destack/db";
import { schema } from "@destack/schema";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/method.ts";
import type { ConstraintColumns, ObjectTable } from "./table.ts";
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
    typeof activityFields,
    { readonly nested: { readonly in: "any"; readonly receive: "activity" } }
>;

/** Define a scope type's activities, one per caller's session of changes. */
export function activity<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "activity",
        plural: "activities",
        scope,
        nested: { in: "any", receive: "activity" },
        fields: activityFields,
        constraints: (entry: ConstraintColumns<"activity", Scope, typeof activityFields>) => [
            index("activity_session").on(
                entry.parentPackageId,
                entry.parentType,
                entry.parentId,
                entry.caller,
                entry.endedAt,
            ),
        ],
        permissions: { read: through("parent", "read") },
        methods: (method: MethodBuilder<ActivityTable>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Define a scope type's checkpoints: named points in an object's history. */
export function checkpoint<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "checkpoint",
        plural: "checkpoints",
        scope,
        nested: { in: "any", receive: "checkpoint" },
        fields: {
            /** The principal who named the checkpoint. */
            createdBy: field.subject().caller(),
            /** The log position the checkpoint names. */
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
        methods: (method: MethodBuilder<ObjectTable>) => ({
            get: method.get("read"),
            list: method.list("read"),
            create: {
                ...method
                    .create("checkpoint", { fields: ["title", "description"] })
                    .handle(async (call, next) =>
                        next(
                            call.with({
                                input: {
                                    ...call.input,
                                    position: await call.database.log.position(),
                                },
                            }),
                        ),
                    ),
                isPredicted: false,
            },
            update: method.update("checkpoint", { fields: ["title", "description"] }),
            delete: method.delete("checkpoint"),
        }),
    } as const;
}
