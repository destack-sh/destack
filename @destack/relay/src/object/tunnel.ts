import { none, principal, relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** Why the relay closes a machine's tunnel: its machine or its last standing key was revoked. */
export const TunnelCloseReason = schema.enum(["machine-revoked", "key-revoked"]);
/** Why the relay closes a machine's tunnel. */
export type TunnelCloseReason = schema.Infer<typeof TunnelCloseReason>;

/** A machine's tunnel to the relay in its residency's home, served by the relay installation. */
export const tunnel = defineObject(
    {
        name: "tunnel",
        plural: "tunnels",
        scope: space,
        isScope: true,
        partition: true,
        fields: {
            /** The name the relay routes to the machine, which its connections renew. */
            name: field.string(schema.string().min(1)),
        },
        relations: {
            // the relay installation serving the tunnel, related as it opens the tunnel
            server: { subjects: [principal.installation], grantedBy: null },
        },
        permissions: {
            read: none(),
            // let the servers of the home's account, its residency's among them, open its tunnels
            open: through("scope", "serve"),
            rename: relation("server"),
            close: relation("server"),
            connect: relation("server"),
        },
    },
    (method) => ({
        /** Record a machine's tunnel as it first opens, served by the installation opening it. */
        open: method.create("open", {
            fields: ["name"],
            creator: "server",
            isInternal: true,
            isPredicted: false,
        }),
        /** Change the name the relay routes to the machine, telling the machine once connected. */
        rename: method.update("rename", { fields: ["name"], isPredicted: false }),
        /** End the tunnel's connections, removing the tunnel of a revoked machine. */
        close: method.mutation({
            permission: "close",
            input: schema.object({ reason: TunnelCloseReason }),
            output: schema.object({ closed: schema.boolean() }),
        }),
    }),
);
