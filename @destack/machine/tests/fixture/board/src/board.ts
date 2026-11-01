import { space } from "@destack/account/object";
import { principal, relation, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** Cards their owner shares with viewers, who pin them in place. */
export const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        fields: {
            /** The principal who created the card. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
        },
        relations: { viewer: { subjects: [principal.user] } },
        permissions: {
            read: union(relation("owner"), relation("viewer")),
            manage: relation("owner"),
        },
        shareable: { by: "manage" },
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
        pin: method.mutation({ permission: "read", input: schema.object({}) }),
    }),
);

/** The cards, whose pins run in the board's sandbox. */
export const cards = card.implement({
    methods: {
        pin: async (call) => call.target,
    },
});
