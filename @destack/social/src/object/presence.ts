import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field, Selection } from "@destack/object";
import { schema } from "@destack/schema";

/** What a present principal does, typing being editing with keystrokes now. */
export const PRESENCE_STATES = ["viewing", "editing", "typing", "idle"] as const;

/** The most bytes of a principal's place outside text, 32 canvas cursors' coordinates in JSON. */
export const LOCATION_BYTES = 1024;

/** One writer's presence on an object, stored in memory. */
export const presence = defineObject({
    name: "presence",
    plural: "presences",
    scope: space,
    storage: "ephemeral",
    nested: { in: "any", receive: "present" },
    fields: {
        /** The principal. */
        principal: field.subject().caller(),
        /** What the principal does. */
        status: field.enum(PRESENCE_STATES),
        /** The principal's selection in the object's text. */
        selection: field.json(Selection).optional(),
        /** The principal's place outside text, such as a canvas cursor. */
        location: field.json(schema.json()).optional(),
    },
    permissions: {
        read: through("parent", "present"),
        write: intersection(relation("principal"), through("parent", "present")),
    },
    constraints: {
        writer: {
            kind: "unique",
            on: ["parentPackageId", "parentType", "parentId", "principalId", "writerId"],
        },
    },
});
