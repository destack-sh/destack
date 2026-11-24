import { space } from "@destack/account/object";
import { principal, relation, union } from "@destack/access";
import { defineAction, defineObject, field, selected } from "@destack/object";
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
            /** The cover. */
            cover: field.file({ accept: ["image/*"] }).optional(),
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
        create: method.create("manage", { fields: ["title", "cover"] }),
        update: method.update("manage", { fields: ["title", "cover"] }),
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
    name: "card.retitleIdeas",
    title: t`Retitle to Ideas`,
    input: { cards: selected(cards, { title: { ne: "Ideas" } }) },
    call: (variables) => variables.cards.prepare("update", { title: "Ideas" }),
});

/** Create a card titled Ideas. */
export const ideas = defineAction({
    name: "card.newIdeas",
    title: t`New Ideas Card`,
    call: () => cards.prepare("create", { title: "Ideas" }),
});
