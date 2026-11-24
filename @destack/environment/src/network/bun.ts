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
    websocket: (url, protocols) =>
        openWebSocket(url, {
            protocols: typeof protocols === "string" ? [protocols] : [...(protocols ?? [])],
        }),
    forward: (upgrade, url) => {
        // offer the upgrade's subprotocols, and send its other headers as Bun's client allows
        const forwarded = WebSocketUpgrade.of(upgrade);
        if (forwarded === undefined) {
            throw new TypeError("a forwarded socket opens for a WebSocket upgrade alone");
        }

        return openWebSocket(url, {
            protocols: forwarded.protocols,
            headers: Object.fromEntries(forwarded.forwarded()),
        });
    },
    resolve: resolveHostname,
};

/** Open a WebSocket through the proxy the environment gives its URL, as Bun's fetch does. */
function openWebSocket(
    url: string,
    options: { readonly protocols: readonly string[]; readonly headers?: Record<string, string> },
): ClientSocket {
    // construct with Bun's options form, which the web's WebSocket type leaves out
    const proxy = proxyOf(new URL(url), process.env);
    const opened: unknown = Reflect.construct(WebSocket, [
        url,
        {
            protocols: [...options.protocols],
            ...(options.headers === undefined ? {} : { headers: options.headers }),
            ...(proxy === undefined ? {} : { proxy }),
        },
    ]);
    if (!(opened instanceof WebSocket)) {
        throw new TypeError("Bun's WebSocket constructor returned no WebSocket");
    }

    return new SystemSocket(opened);
}

/** Read a URL's proxy: `HTTP_PROXY` for `ws:`, `HTTPS_PROXY` for `wss:`, minus `NO_PROXY`. */
function proxyOf(url: URL, variables: Record<string, string | undefined>): string | undefined {
    // read the variable of the URL's scheme, either case
    const name = url.protocol === "wss:" ? "https_proxy" : "http_proxy";
    const proxy = variables[name] ?? variables[name.toUpperCase()];
    if (proxy === undefined || proxy === "") {
        return undefined;
    }

    // leave a host that `NO_PROXY` lists, by exact name or domain suffix, or every host for `*`
    const excluded = (variables["no_proxy"] ?? variables["NO_PROXY"] ?? "")
        .split(",")
        .map((entry) => entry.trim().replace(/^\./u, "").toLowerCase())
        .filter((entry) => entry !== "");
    const host = url.hostname.toLowerCase();
    const isExcluded = excluded.some(
        (entry) => entry === "*" || host === entry || host.endsWith(`.${entry}`),
    );

    return isExcluded ? undefined : proxy;
}
