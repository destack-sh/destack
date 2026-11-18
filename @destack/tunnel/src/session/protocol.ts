import { BearerProtocol, type WebSocketUpgrade } from "@destack/environment";
import { schema } from "@destack/schema";

/** The WebSocket subprotocol of a machine's tunnel, the only one the relay answers with. */
export const TUNNEL_PROTOCOL = "destack.tunnel";

/** The text message a machine probes its tunnel's liveness with. */
export const PING_MESSAGE = "ping";

/** The text message the relay answers a liveness probe with. */
export const PONG_MESSAGE = "pong";

/** The subprotocols a machine opens its tunnel with. */
export const TunnelProtocol = {
    /** List the tunnel protocol and the bearer token a machine offers. */
    offer(token: string): string[] {
        return [TUNNEL_PROTOCOL, BearerProtocol.write(token)];
    },

    /** Read the bearer token an upgrade offers beside the tunnel protocol. */
    token(upgrade: WebSocketUpgrade): string | undefined {
        return upgrade.protocols.includes(TUNNEL_PROTOCOL)
            ? BearerProtocol.read(upgrade.request.headers)
            : undefined;
    },
};

/** The answer to a machine's renewal: its name and when its new token lapses. */
export const Renewal = schema.object({
    /** The name the relay routes to the machine, such as `laptop--florian.destack.computer`. */
    name: schema.string().min(1),
    /** When the renewed token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number().int(),
});
/** The answer to a machine's renewal. */
export type Renewal = schema.Infer<typeof Renewal>;
