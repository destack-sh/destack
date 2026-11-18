import type { ServerWebSocket } from "bun";
import type { Listener, ServeOptions, Server, ServerSocket } from "./server.ts";
import { listenSocket } from "../network/socket/bun.ts";

/** The media type of a response that streams events for as long as its client listens. */
const EVENT_STREAM = "text/event-stream";

/** The status of a refused WebSocket upgrade, Upgrade Required (RFC 9110 15.5.22). */
const UPGRADE_REQUIRED = 426;

/** The status of a send Bun dropped, as the socket closed or its buffer is past its limit. */
const SEND_DROPPED = 0;

/** The status of a send Bun buffered behind earlier ones, until the socket drains. */
const SEND_BUFFERED = -1;

/** What Bun stores beside a socket: its attachment, and its wrapper once made. */
interface BunSocketData<Attachment> {
    /** The value the handler accepted the upgrade with. */
    readonly attachment: Attachment;
    /** The socket as its handler sees it, made at its first event. */
    wrapped: ServerSocket<Attachment> | undefined;
    /** The settlers of the next drain, while sends wait for the socket's buffer. */
    drain: PromiseWithResolvers<void> | undefined;
}

/** A Bun WebSocket with its data. */
type BunSocket<Attachment> = ServerWebSocket<BunSocketData<Attachment>>;

/** Listen through Bun's server, refusing a runtime without one. */
function serveHttp<Attachment>(options: ServeOptions<Attachment>): Listener {
    // answer each request, or upgrade it to the WebSocket its handler accepts
    const { fetch, websocket } = options;
    const handler = {
        fetch: async (request: Request, served: Bun.Server<BunSocketData<Attachment>>) => {
            // answer a response
            const answer = await fetch(request, served.requestIP(request)?.address);
            if (answer instanceof Response) {
                // leave an event stream open past Bun's 10 s idle timeout, as Bun documents
                if (answer.headers.get("content-type")?.startsWith(EVENT_STREAM) === true) {
                    served.timeout(request, 0);
                }

                return answer;
            }

            // upgrade with the attachment, refusing a request Bun cannot upgrade
            const data = { attachment: answer.attachment, wrapped: undefined, drain: undefined };
            const headers = answer.headers === undefined ? {} : { headers: answer.headers };
            const isUpgraded = served.upgrade(request, { data, ...headers });

            return isUpgraded ? undefined : new Response(null, { status: UPGRADE_REQUIRED });
        },
        websocket: {
            ...(websocket?.maxPayloadLength === undefined
                ? {}
                : { maxPayloadLength: websocket.maxPayloadLength }),
            open: (socket: BunSocket<Attachment>) => websocket?.open?.(wrapSocket(socket)),
            message: (socket: BunSocket<Attachment>, message: string | Uint8Array) =>
                websocket?.message(wrapSocket(socket), message),
            drain: (socket: BunSocket<Attachment>) => {
                // settle the sends waiting for the buffer
                socket.data.drain?.resolve();
                socket.data.drain = undefined;
            },
            close: (socket: BunSocket<Attachment>, code: number, reason: string) => {
                // fail the sends waiting for a drain, then tell the handler
                socket.data.drain?.reject(new TypeError("the socket closed before it drained"));
                socket.data.drain = undefined;
                websocket?.close?.(wrapSocket(socket), code, reason);
            },
        },
    };

    // listen on the Unix socket or the host name and port
    const server =
        options.unix === undefined
            ? Bun.serve<BunSocketData<Attachment>>({
                  ...(options.hostname === undefined ? {} : { hostname: options.hostname }),
                  ...(options.port === undefined ? {} : { port: options.port }),
                  ...handler,
              })
            : Bun.serve<BunSocketData<Attachment>>({ unix: options.unix, ...handler });

    return { url: server.url, stop: (isClosingActive) => server.stop(isClosingActive) };
}

/** Wrap a Bun socket once, as its handler sees it, beside its attachment. */
function wrapSocket<Attachment>(socket: BunSocket<Attachment>): ServerSocket<Attachment> {
    socket.data.wrapped ??= {
        attachment: socket.data.attachment,
        send: async (message) => {
            // send, refusing a dropped message, and wait for a drain while Bun buffers it
            const status = socket.send(message);
            if (status === SEND_DROPPED) {
                throw new TypeError(
                    "the socket dropped a message: it closed or its buffer is full",
                );
            } else if (status === SEND_BUFFERED) {
                socket.data.drain ??= Promise.withResolvers();
                await socket.data.drain.promise;
            }
        },
        close: (code, reason) => socket.close(code, reason),
    };

    return socket.data.wrapped;
}

/** Accept requests, WebSockets and stream sockets through Bun's servers. */
export const BunServer: Server = { http: serveHttp, socket: listenSocket };
