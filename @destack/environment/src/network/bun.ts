import { checkServerIdentity } from "node:tls";
import { Network } from "./network.ts";
import { copyRequest } from "./request.ts";
import { connectSocket, resolveHostname } from "./socket/bun.ts";

/** Bun's network: the web's, with client certificates, Unix sockets and TCP connections. */
export const BunNetwork: Network = {
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
    resolve: resolveHostname,
};
