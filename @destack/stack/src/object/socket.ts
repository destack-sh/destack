import { relation, through, union } from "@destack/access";
import { client, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { type ObjectOf, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { installation } from "./installation.ts";

/** The kinds of live links to a space: a browser tab, a desktop view, a machine or a workload. */
export const SOCKET_KINDS = ["tab", "view", "machine", "workload"] as const;

/** A kind of live link to a space. */
export type SocketKind = (typeof SOCKET_KINDS)[number];

/** One live link to a space while its socket stays open. */
export const socket = defineObject({
    name: "socket",
    plural: "sockets",
    scope: space,
    storage: "ephemeral",
    fields: {
        /** What the link connects. */
        kind: field.enum(SOCKET_KINDS),
        /** The principal connected. */
        principal: field.subject(),
        /** The client the socket's credential is bound to, absent for an unbound credential. */
        client: field.reference(client).optional().guard({ read: "inspect" }),
        /** The session the socket authenticated with, as access decisions key it. */
        session: field
            .string(schema.string().min(1).max(256))
            .optional()
            .guard({ read: "inspect" }),
        /** The installation connected, such as the one serving a view. */
        installation: field
            .reference((): ObjectOf<{ identity: "installation" }> => installation)
            .optional(),
        /** The last heartbeat of the socket. */
        lastActiveAt: field.time(),
    },
    permissions: {
        read: union(relation("principal"), through("space", "member")),
        inspect: union(relation("principal"), through("space", "manage")),
    },
}).methods();

/** A live socket as its memory table keeps it. */
export type Socket = Select<typeof socket.table>;
