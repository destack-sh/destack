import { Network } from "./network.ts";
import { connectTcp } from "./tcp/bun.ts";

/** Bun's network: the web's, with client certificates, Unix sockets and TCP connections. */
export const BunNetwork: Network = {
    fetch: (request, options) => {
        // send through the web's fetch without host options
        if (options?.certificate === undefined && options?.unix === undefined) {
            return fetch(request);
        }

        // present the certificate and send through the socket the options give
        const { certificate, unix } = options;

        return fetch(request, {
            ...(certificate === undefined
                ? {}
                : { tls: { cert: certificate.certificate, key: certificate.privateKey } }),
            ...(unix === undefined ? {} : { unix }),
        });
    },
    connect: connectTcp,
    websocket: Network.system.websocket,
};
