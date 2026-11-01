import { space } from "@destack/account/object";
import { intersection, principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** What a channel member's presence shows. */
const PRESENCE_STATES = ["viewing", "typing", "idle"] as const;

/** A member's presence on an object, kept in memory while they are connected. */
export const presence = defineObject(
    {
        name: "presence",
        plural: "presences",
        scope: space,
        storage: "ephemeral",
        nested: { in: "any", receive: "present" },
        fields: {
            /** The present principal. */
            principal: field.subject().caller(),
            /** What the principal does. */
            status: field.enum(PRESENCE_STATES),
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
        scope: space,
        fields: {
            /** The principal who created the channel. */
            owner: field.subject().caller(),
            /** The channel's name. */
            name: field.string(schema.string().min(1).max(100)),
        },
        relations: { member: { subjects: [principal.user] } },
        permissions: {
            read: union(relation("owner"), relation("member")),
            write: union(relation("owner"), relation("member")),
            manage: relation("owner"),
        },
        shareable: { by: "manage" },
        attachments: [presence.attach({ by: "read" })],
    },
    (method) => ({ create: method.create("manage", { fields: ["name"] }) }),
);

/** A message posted to a channel. */
export const message = defineObject(
    {
        name: "message",
        plural: "messages",
        scope: space,
        nested: { in: channel, delete: "cascade", receive: "write" },
        fields: {
            /** The principal who posted the message. */
            author: field.subject().caller(),
            /** The message's text. */
            text: field.string(schema.string().min(1).max(4000)),
        },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({ create: method.create("write", { fields: ["text"] }) }),
);
