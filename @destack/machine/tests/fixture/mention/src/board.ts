import { space } from "@destack/account/object";
import { principal, relation, union } from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { activity, announcement, subscription } from "@destack/notification/object";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { shareable } from "@destack/object";

/** Someone mentions the recipient on a card. */
export const mention = defineNotification({
    name: "mention",
    title: t`Mentions`,
    description: t`Someone mentions you on a card.`,
    payload: schema.object({ text: schema.string() }),
    interruption: "active",
    preference: { channels: ["desktop"], delivery: "immediate" },
    content: { title: t`Mentioned`, body: t`${argument("text")}` },
    summary: t`${plural(argument("count"), { one: "# mention", other: "# mentions" })}`,
});

/** Cards their owner shares with viewers, mentioning people in remarks. */
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
        attachments: [
            activity.attach({ by: "read" }),
            announcement.attach({ by: "read" }),
            subscription.attach({ by: "read" }),
        ],
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
        remark: method.mutation({
            title: t`Remark`,
            permission: "read",
            input: schema.object({ text: schema.string(), mentions: schema.array(Subject) }),
        }),
    }),
);

/** The cards, notifying the people a remark mentions. */
export const cards = card.implement({
    methods: {
        remark: async (call) => {
            // notify the people the remark mentions
            await call.mutate(call.caller, [
                mention.call({
                    source: call.reference(),
                    recipients: call.input.mentions,
                    reason: "mention",
                    payload: { text: call.input.text },
                }),
            ]);

            return call.target;
        },
    },
});
