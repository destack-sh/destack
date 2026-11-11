import { space } from "@destack/account/object";
import { intersection, principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { nested, shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** What a channel member's presence shows. */
const PRESENCE_STATES = ["viewing", "typing", "idle"] as const;

/** A member's presence on an object, kept in memory while they are connected. */
export const presence = defineObject(
    {
        name: "presence",
        plural: "presences",
        title: t`${plural(argument("count"), { one: "Presence", other: "Presences" })}`,
        scope: space,
        storage: "ephemeral",
        fields: {
            /** The present principal. */
            principal: field.subject().caller(),
            /** What the principal does. */
            status: field.enum(PRESENCE_STATES),
        },
        constraints: {
            writer: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "principalId", "writerId"],
            },
        },
        access: {
            permissions: {
                read: through("parent", "present"),
                write: intersection(relation("principal"), through("parent", "present")),
            },
        },
        traits: [nested({ in: "any", add: "present" })],
    },
    (method) => ({
        create: method.create("write", { fields: ["status"] }),
        update: method.update("write", { fields: ["status"] }),
    }),
);

/** A chat channel, whose members read and post its messages. */
export const channel = defineObject(
    {
        name: "channel",
        plural: "channels",
        title: t`${plural(argument("count"), { one: "Channel", other: "Channels" })}`,
        scope: space,
        fields: {
            /** The principal who created the channel. */
            owner: field.subject().caller(),
            /** The channel's name. */
            name: field.string(schema.string().min(1).max(100)),
        },
        attachments: [presence.attach({ by: "read" })],
        access: {
            relations: { member: { subjects: [principal.user] } },
            permissions: {
                read: union(relation("owner"), relation("member")),
                write: union(relation("owner"), relation("member")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({ create: method.create("manage", { fields: ["name"] }) }),
);

/** A message posted to a channel. */
export const message = defineObject(
    {
        name: "message",
        plural: "messages",
        title: t`${plural(argument("count"), { one: "Message", other: "Messages" })}`,
        scope: space,
        fields: {
            /** The principal who posted the message. */
            author: field.subject().caller(),
            /** The message's text. */
            text: field.string(schema.string().min(1).max(4000)),
        },
        access: {
            permissions: { read: through("parent", "read"), write: through("parent", "write") },
        },
        traits: [nested({ in: channel, delete: "cascade", add: "write" })],
    },
    (method) => ({ create: method.create("write", { fields: ["text"] }) }),
);
