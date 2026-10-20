import { through } from "@destack/access";
import { LogPosition } from "@destack/db";
import { schema } from "@destack/schema";
import { field, type FieldsByColumn } from "../field/field.ts";
import type { ObjectTable } from "../object/table.ts";
import { defineObject, type ObjectScope } from "../object/object.ts";

/** The fields of an activity: one caller's session of changes to an object. */
const activityFields = {
    /** The principal whose changes the session records. */
    caller: field.subject().caller(),
    /** When the session's first change committed, in UTC epoch milliseconds. */
    startedAt: field.time(),
    /** When its latest change committed, in UTC epoch milliseconds. */
    endedAt: field.time(),
    /** The log position before the session. */
    from: field.json(LogPosition),
    /** The fields the session changed. */
    fields: field.json(schema.array(schema.string())),
    /** How many changes the session records. */
    changes: field.integer(),
};

/** The table of activities, nested under the objects whose changes they record. */
export type ActivityTable = ObjectTable<
    "activity",
    ObjectScope,
    FieldsByColumn<typeof activityFields>,
    { readonly nested: { readonly in: "any"; readonly receive: "activity" } }
>;

/** The activities of a scope, one per caller's session of changes to an object. */
export const activity = defineObject({
    name: "activity",
    plural: "activities",
    nested: { in: "any", receive: "activity" },
    fields: activityFields,
    permissions: { read: through("parent", "read") },
    constraints: {
        session: {
            kind: "index",
            on: ["parentPackageId", "parentType", "parentId", "callerId", "endedAt"],
        },
    },
});
