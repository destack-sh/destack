import type { ClientCertificate } from "../network/network.ts";

/** Send a request with a TLS client certificate through Bun, refusing other runtimes. */
export function fetchBun(request: Request, certificate: ClientCertificate): Promise<Response> {
    // refuse runtimes that cannot present a certificate
    if (typeof Bun === "undefined") {
        throw new TypeError("presenting a client certificate requires the Bun runtime");
    }

    return fetch(request, { tls: { cert: certificate.certificate, key: certificate.privateKey } });
}
