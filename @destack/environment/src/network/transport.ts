import { type Attributes, type Span, SpanStatusCode } from "@opentelemetry/api";
import { SpanRunner } from "../telemetry/runner.ts";
import { TraceHeaders } from "../telemetry/headers.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { ClientSocket, FetchOptions, Network } from "./network.ts";
import { copyRequest } from "./request.ts";
import type { ConnectOptions, Socket, SocketAddress } from "./socket/socket.ts";

/** The instrumentation scope of HTTP spans and durations: this package. */
export const HTTP_SCOPE = "@destack/environment";

/** The lowest status a client counts as failed: any 4xx or 5xx, as OpenTelemetry sets. */
const CLIENT_ERROR_STATUS = 400;

/** The default ports of URL schemes, which a URL without a port uses. */
const DEFAULT_PORTS: Readonly<Record<string, number>> = { "http:": 80, "https:": 443 };

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
            telemetry.tracers.getTracer(HTTP_SCOPE),
            telemetry.meters.getMeter(HTTP_SCOPE),
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
        const bounded = {
            "http.request.method": request.method,
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
            request.method,
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

    /** Open a connection through the source network. */
    readonly connect = (address: SocketAddress, options?: ConnectOptions): Socket =>
        this.source.connect(address, options);

    /** Open a WebSocket through the source network. */
    readonly websocket = (url: string, protocols?: string | readonly string[]): ClientSocket =>
        this.source.websocket(url, protocols);

    /** Resolve a host name through the source network. */
    readonly resolve = (hostname: string): Promise<readonly string[]> =>
        this.source.resolve(hostname);
}
