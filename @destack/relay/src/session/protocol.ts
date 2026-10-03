/** The WebSocket subprotocol of a host's tunnel, the only one the relay answers with. */
export const TUNNEL_PROTOCOL = "destack.tunnel";

/** The prefix of the subprotocol carrying a host's bearer token in base64url. */
const BEARER_PREFIX = "destack.bearer.";

/** The subprotocols a host opens its tunnel with. */
export const TunnelProtocol = {
    /** List the tunnel protocol and the bearer token a host offers. */
    offer(token: string): string[] {
        const encoded = new TextEncoder()
            .encode(token)
            .toBase64({ alphabet: "base64url", omitPadding: true });

        return [TUNNEL_PROTOCOL, `${BEARER_PREFIX}${encoded}`];
    },

    /** Read the bearer token a `Sec-WebSocket-Protocol` header offers, absent for none or an invalid one. */
    token(offered: string | null): string | undefined {
        // require the tunnel protocol and one bearer token
        const protocols = offered === null ? [] : offered.split(",").map((entry) => entry.trim());
        const bearers = protocols.filter((protocol) => protocol.startsWith(BEARER_PREFIX));
        const bearer = bearers.length === 1 ? bearers[0] : undefined;
        if (!protocols.includes(TUNNEL_PROTOCOL) || bearer === undefined) {
            return undefined;
        }

        // decode the token, refusing invalid base64url
        try {
            const encoded = bearer.slice(BEARER_PREFIX.length);
            const bytes = Uint8Array.fromBase64(encoded, { alphabet: "base64url" });

            return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        } catch {
            return undefined;
        }
    },
};
