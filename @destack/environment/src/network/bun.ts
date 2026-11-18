import { checkServerIdentity } from "node:tls";
import { type ClientSocket, Network, SystemSocket } from "./network.ts";
import { copyRequest } from "./request.ts";
import { WebSocketUpgrade } from "./websocket.ts";
import { connectSocket, resolveHostname } from "./socket/bun.ts";

/** Bun's network: the web's, with client certificates, Unix sockets and TCP connections. */
export const BunNetwork: Network & {
    /** Open a WebSocket at a URL as a forwarded upgrade, with its headers. */
    readonly forward: (upgrade: Request, url: string) => ClientSocket;
} = {
    origin: undefined,
    fetch: async (request, options) => {
        // present the certificate, send through the socket, and keep the encoding the options give
        const { certificate, unix, decompress, isPublicOnly } = options ?? {};
        const presented =
            certificate === undefined
                ? {}
                : { cert: certificate.certificate, key: certificate.privateKey };
        const sent = {
            ...(unix === undefined ? {} : { unix }),
            ...(decompress === undefined ? {} : { decompress }),
        };
        if (isPublicOnly !== true) {
            return fetch(request, { ...sent, tls: presented });
        }

        // send a public host's request to the address checked, under its name for the host header and the certificate
        const url = new URL(request.url);
        const address = await Network.requirePublic(BunNetwork, url.hostname);
        const pinned = new URL(request.url);
        pinned.hostname = address.includes(":") ? `[${address}]` : address;
        const headers = new Headers(request.headers);
        headers.set("host", url.host);

        return fetch(copyRequest(request, { headers }, pinned.href), {
            ...sent,
            tls: {
                ...presented,
                serverName: url.hostname,
                checkServerIdentity: (_host, peer) => checkServerIdentity(url.hostname, peer),
            },
        });
    },
    connect: connectSocket,
    websocket: Network.system.websocket,
    forward: (upgrade, url) => {
        // offer the upgrade's subprotocols, and send its other headers as Bun's client allows
        const forwarded = WebSocketUpgrade.of(upgrade);
        if (forwarded === undefined) {
            throw new TypeError("a forwarded socket opens for a WebSocket upgrade alone");
        }

        // construct with Bun's options form, which the web's WebSocket type leaves out
        const opened: unknown = Reflect.construct(WebSocket, [
            url,
            {
                protocols: forwarded.protocols,
                headers: Object.fromEntries(forwarded.forwarded()),
            },
        ]);
        if (!(opened instanceof WebSocket)) {
            throw new TypeError("Bun's WebSocket constructor returned no WebSocket");
        }

        return new SystemSocket(opened);
    },
    resolve: resolveHostname,
};
