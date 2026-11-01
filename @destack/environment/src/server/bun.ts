import type { ServerWebSocket } from "bun";
import type { Listener, ServeOptions, Server, ServerSocket } from "./server.ts";
import { listenSocket } from "../network/socket/bun.ts";

/** The status of a refused WebSocket upgrade, Upgrade Required (RFC 9110 15.5.22). */
const UPGRADE_REQUIRED = 426;

/** What Bun stores beside a socket: its attachment, and its wrapper once made. */
interface BunSocketData<Attachment> {
    /** The value the handler accepted the upgrade with. */
    readonly attachment: Attachment;
    /** The socket as its handler sees it, made at its first event. */
    wrapped: ServerSocket<Attachment> | undefined;
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
                return answer;
            }

            // upgrade with the attachment, refusing a request Bun cannot upgrade
            const data = { attachment: answer.attachment, wrapped: undefined };
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
            close: (socket: BunSocket<Attachment>, code: number, reason: string) =>
                websocket?.close?.(wrapSocket(socket), code, reason),
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
        send: (message) => void socket.send(message),
        close: (code, reason) => socket.close(code, reason),
    };

    return socket.data.wrapped;
}

/** Accept requests, WebSockets and stream sockets through Bun's servers. */
export const BunServer: Server = { http: serveHttp, socket: listenSocket };
