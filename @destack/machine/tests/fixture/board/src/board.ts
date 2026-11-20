import { space } from "@destack/account/object";
import { principal, relation, union } from "@destack/access";
import { defineAction, defineObject, field } from "@destack/object";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { shareable } from "@destack/object";

/** Cards their owner shares with viewers, who pin them in place. */
export const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: space,
        fields: {
            /** The principal who created the card. */
            owner: field.subject().caller(),
            /** The title. */
            title: field.string(schema.string().min(1)),
        },
        access: {
            relations: { viewer: { subjects: [principal.user] } },
            permissions: {
                read: union(relation("owner"), relation("viewer")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({
        create: method.create("manage", { fields: ["title"] }),
        update: method.update("manage", { fields: ["title"] }),
        pin: method.mutation({ title: t`Pin`, permission: "read", input: schema.object({}) }),
    }),
);

/** The cards, whose pins run in the board's sandbox. */
export const cards = card.implement({
    methods: {
        pin: async (call) => call.target,
    },
});

/** Retitle a card Ideas, for cards of another title. */
export const retitle = defineAction({
    name: "retitle-ideas",
    title: t`Retitle to Ideas`,
    call: {
        object: cards,
        method: "update",
        input: { title: "Ideas" },
        when: { title: { ne: "Ideas" } },
    },
});

/** Create a card titled Ideas. */
export const ideas = defineAction({
    name: "new-ideas",
    title: t`New Ideas Card`,
    call: { object: cards, method: "create", input: { title: "Ideas" } },
});
