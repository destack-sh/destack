import { Network } from "./network.ts";
import { connectSocket } from "./socket/bun.ts";

/** Bun's network: the web's, with client certificates, Unix sockets and TCP connections. */
export const BunNetwork: Network = {
    fetch: (request, options) => {
        // present the certificate, send through the socket, and keep the encoding the options give
        const { certificate, unix, decompress } = options ?? {};

        return fetch(request, {
            ...(certificate === undefined
                ? {}
                : { tls: { cert: certificate.certificate, key: certificate.privateKey } }),
            ...(unix === undefined ? {} : { unix }),
            ...(decompress === undefined ? {} : { decompress }),
        });
    },
    connect: connectSocket,
    websocket: Network.system.websocket,
};
