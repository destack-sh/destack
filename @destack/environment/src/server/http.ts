import type { Attributes } from "@opentelemetry/api";
import { HTTP_SCOPE, HttpMethod, HttpTransport } from "../network/transport.ts";
import type { SocketListener, SocketListenOptions } from "../network/socket/socket.ts";
import { SpanRunner } from "../telemetry/runner.ts";
import { TraceHeaders } from "../telemetry/headers.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { Listener, ServeOptions, Server } from "./server.ts";

/** The lowest status a server counts as its own failure: any 5xx, as OpenTelemetry sets. */
const SERVER_ERROR_STATUS = 500;

/** The status of an accepted WebSocket upgrade, Switching Protocols (RFC 9110 15.2.2). */
const SWITCHING_PROTOCOLS = 101;

/** A server answering each request in an `http.server` span below the trace it carries. */
export class HttpServer implements Server {
    /** The server that accepts the connections. */
    readonly source: Server;
    /** The telemetry the spans, durations and trace headers come from. */
    readonly #telemetry: Telemetry;
    /** The runner of the requests' spans. */
    readonly #span: SpanRunner;

    /** Trace a server's requests on a telemetry, as OpenTelemetry's HTTP server conventions do. */
    constructor(source: Server, telemetry: Telemetry) {
        this.source = source;
        this.#telemetry = telemetry;
        this.#span = SpanRunner.of(
            telemetry.tracers.getTracer(HTTP_SCOPE),
            telemetry.meters.getMeter(HTTP_SCOPE),
            telemetry,
        );
    }

    /** Read the untraced server below a server, itself when untraced. */
    static sourceOf(server: Server): Server {
        return server instanceof HttpServer ? server.source : server;
    }

    /** Accept requests, answering each in its span below the trace its headers carry. */
    readonly http = <Attachment>(options: ServeOptions<Attachment>): Listener =>
        this.source.http({
            ...options,
            fetch: (request, peer) => {
                // describe the request by its method and scheme, its path and peer on the span alone
                const url = new URL(request.url);
                const method = HttpMethod.of(request.method);
                const bounded = {
                    "http.request.method": method,
                    "url.scheme": url.protocol.slice(0, -1),
                };
                const attributes = {
                    ...bounded,
                    "url.path": url.pathname,
                    ...(peer === undefined ? {} : { "client.address": peer }),
                };
                let outcome: Attributes = {};

                // answer it below the trace its headers carry, noting the status or the upgrade
                const parent = TraceHeaders.extract(request.headers, this.#telemetry);
                const answer = () =>
                    this.#span(
                        method,
                        attributes,
                        async (span) => {
                            // answer the request, an accepted upgrade answering no response
                            const answered = await options.fetch(request, peer);
                            const status =
                                answered instanceof Response
                                    ? answered.status
                                    : SWITCHING_PROTOCOLS;
                            outcome = HttpTransport.answered(span, status, SERVER_ERROR_STATUS);

                            return answered;
                        },
                        {
                            kind: "server",
                            duration: {
                                name: "http.server.request.duration",
                                attributes: () => ({ ...bounded, ...outcome }),
                            },
                        },
                    );

                return this.#telemetry.context.with(parent, answer);
            },
        });

    /** Accept stream sockets through the source server. */
    readonly socket = (options: SocketListenOptions): Promise<SocketListener> =>
        this.source.socket(options);
}
