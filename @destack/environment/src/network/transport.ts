import { type Attributes, type Span, SpanStatusCode } from "@opentelemetry/api";
import { ENVIRONMENT_SCOPE, SpanRunner } from "../telemetry/runner.ts";
import { TraceHeaders } from "../telemetry/headers.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { ClientSocket, FetchOptions, Network } from "./network.ts";
import { copyRequest } from "./request.ts";
import type { ConnectOptions, Socket, SocketAddress } from "./socket/socket.ts";

/** The HTTP methods of RFC 9110 and RFC 5789, which spans and durations state as they are. */
const HTTP_METHODS: ReadonlySet<string> = new Set([
    "CONNECT",
    "DELETE",
    "GET",
    "HEAD",
    "OPTIONS",
    "PATCH",
    "POST",
    "PUT",
    "TRACE",
]);

/** The lowest status a client counts as failed: any 4xx or 5xx, as OpenTelemetry sets. */
const CLIENT_ERROR_STATUS = 400;

/** The default ports of URL schemes, which a URL without a port uses. */
const DEFAULT_PORTS: Readonly<Record<string, number>> = { "http:": 80, "https:": 443 };

/** HTTP methods as OpenTelemetry reports them. */
export const HttpMethod = {
    /** Read a request's method, any method RFC 9110 and RFC 5789 do not define as `_OTHER`. */
    of(method: string): string {
        return HTTP_METHODS.has(method) ? method : "_OTHER";
    },
};

/** A network sending each request in an `http.client` span with its trace, as otelhttp does. */
export class HttpTransport implements Network {
    /** The network the requests go through. */
    readonly source: Network;
    /** The telemetry the spans, durations and trace headers come from. */
    readonly #telemetry: Telemetry;
    /** The runner of the requests' spans. */
    readonly #span: SpanRunner;

    /** Trace a network's requests on a telemetry, as OpenTelemetry's HTTP client conventions do. */
    constructor(source: Network, telemetry: Telemetry) {
        this.source = source;
        this.#telemetry = telemetry;
        this.#span = SpanRunner.of(
            telemetry.tracers.getTracer(ENVIRONMENT_SCOPE),
            telemetry.meters.getMeter(ENVIRONMENT_SCOPE),
            telemetry,
        );
    }

    /** Read the untraced network below a network, itself when untraced. */
    static sourceOf(network: Network): Network {
        return network instanceof HttpTransport ? network.source : network;
    }

    /** Read a URL's port, its scheme's default when it gives none. */
    static port(url: URL): number {
        return url.port === "" ? (DEFAULT_PORTS[url.protocol] ?? 0) : Number(url.port);
    }

    /** Note an answered status on a span, failing it from a lowest failing status. */
    static answered(span: Span, status: number, failing: number): Attributes {
        // describe the status, and its failure from the lowest failing status
        const failed = status >= failing ? { "error.type": String(status) } : {};
        const outcome = { "http.response.status_code": status, ...failed };

        // note it on the span, failing the span with it
        span.setAttributes(outcome);
        if (status >= failing) {
            span.setStatus({ code: SpanStatusCode.ERROR });
        }

        return outcome;
    }

    /** Send a request in its span with its trace, its duration by method, scheme and status. */
    readonly fetch = (request: Request, options?: FetchOptions): Promise<Response> => {
        // describe the request by its method and scheme, its server and URL on the span alone
        const url = new URL(request.url);
        const method = HttpMethod.of(request.method);
        const bounded = {
            "http.request.method": method,
            "url.scheme": url.protocol.slice(0, -1),
        };
        const attributes = {
            ...bounded,
            "server.address": url.hostname,
            "server.port": HttpTransport.port(url),
            "url.full": `${url.origin}${url.pathname}`,
        };
        let outcome: Attributes = {};

        // send it with its trace in its headers, noting the status it answers
        return this.#span(
            method,
            attributes,
            async (span) => {
                // send the request with the trace in a copy of its headers
                const headers = new Headers(request.headers);
                TraceHeaders.inject(headers, this.#telemetry);
                const copy = copyRequest(request, { headers });
                const response = await this.source.fetch(copy, options);
                outcome = HttpTransport.answered(span, response.status, CLIENT_ERROR_STATUS);

                return response;
            },
            {
                kind: "client",
                duration: {
                    name: "http.client.request.duration",
                    attributes: () => ({ ...bounded, ...outcome }),
                },
            },
        );
    };

    /** Open a connection through the source network, in a span until it opens. */
    readonly connect = (address: SocketAddress, options?: ConnectOptions): Socket => {
        // start the span now, ending it once the socket opens or fails to
        const socket = this.source.connect(address, options);
        const server =
            "unix" in address
                ? { "server.address": address.unix }
                : { "server.address": address.hostname, "server.port": address.port };
        const opened = this.#span("destack.network.connect", server, () => socket.opened);

        // leave the failure to the socket's owner, as the span records it
        opened.catch(() => undefined);

        return socket;
    };

    /** Open a WebSocket through the source network. */
    readonly websocket = (url: string, protocols?: string | readonly string[]): ClientSocket =>
        this.source.websocket(url, protocols);

    /** Resolve a host name in a `dns.lookup` span, as OpenTelemetry names it. */
    readonly resolve = (hostname: string): Promise<readonly string[]> =>
        this.#span("dns.lookup", { "dns.question.name": hostname }, () =>
            this.source.resolve(hostname),
        );
}
