import { space } from "@destack/account/object";
import { intersection, relation, through } from "@destack/access";
import { defineObject, field, Selection } from "@destack/object";
import { schema } from "@destack/schema";

/** One writer's presence on an object, kept in memory. */
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
        status: field.enum(["viewing", "editing", "idle"]),
        /** The principal's selection in the object's text. */
        selection: field.json(Selection).optional(),
        /** The principal's place outside text, such as a canvas cursor. */
        location: field.json(schema.json()).optional(),
        /** Whether the principal is typing. */
        isTyping: field.boolean().default(false),
    },
    permissions: {
        read: through("parent", "present"),
        write: intersection(relation("principal"), through("parent", "present")),
    },
    constraints: {
        writer: {
            kind: "unique",
            on: ["parentPackageId", "parentType", "parentId", "principal", "writerId"],
        },
    },
}).methods((method) => ({
    list: method.list("read"),
    create: method.create("write"),
    update: method.update("write"),
    delete: method.delete("write"),
}));
