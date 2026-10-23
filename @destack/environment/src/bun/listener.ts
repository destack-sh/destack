import type { ServerWebSocket } from "bun";
import type { Listener, ServeOptions, ServerSocket } from "../network/network.ts";

/** What Bun stores beside a socket: the key of its attachment in the listener. */
interface BunSocketKey {
    /** The socket's key. */
    readonly key: number;
}

/** Listen through Bun's server, refusing a runtime without one. */
export function serveBun<Attachment>(options: ServeOptions<Attachment>): Listener<Attachment> {
    // refuse runtimes that cannot listen
    if (typeof Bun === "undefined") {
        throw new TypeError("listening requires the Bun runtime");
    }

    // keep each accepted socket's attachment, and wrap each Bun socket once
    const attachments = new Map<number, { readonly attachment: Attachment }>();
    const sockets = new WeakMap<ServerWebSocket<BunSocketKey>, ServerSocket<Attachment>>();
    let next = 0;
    const wrap = (socket: ServerWebSocket<BunSocketKey>): ServerSocket<Attachment> => {
        // reuse the socket's wrapper
        const known = sockets.get(socket);
        if (known !== undefined) {
            return known;
        }

        // wrap the socket with its attachment
        const kept = attachments.get(socket.data.key);
        if (kept === undefined) {
            throw new TypeError(`no attachment for socket ${socket.data.key}`);
        }
        const wrapped: ServerSocket<Attachment> = {
            attachment: kept.attachment,
            send: (message) => void socket.send(message),
            close: (code, reason) => socket.close(code, reason),
        };
        sockets.set(socket, wrapped);

        return wrapped;
    };

    // answer requests and sockets through Bun, handing each handler the listener
    const { fetch, websocket } = options;
    const handler = {
        fetch: (request: Request) => fetch(request, listener),
        websocket: {
            ...(websocket?.maxPayloadLength === undefined
                ? {}
                : { maxPayloadLength: websocket.maxPayloadLength }),
            open: (socket: ServerWebSocket<BunSocketKey>) => websocket?.open?.(wrap(socket)),
            message: (socket: ServerWebSocket<BunSocketKey>, message: string | Uint8Array) =>
                websocket?.message(wrap(socket), message),
            close: (socket: ServerWebSocket<BunSocketKey>, code: number, reason: string) => {
                websocket?.close?.(wrap(socket), code, reason);
                attachments.delete(socket.data.key);
            },
        },
    };

    // listen on the Unix socket or the host name and port
    const server =
        options.unix === undefined
            ? Bun.serve<BunSocketKey>({
                  ...(options.hostname === undefined ? {} : { hostname: options.hostname }),
                  ...(options.port === undefined ? {} : { port: options.port }),
                  ...handler,
              })
            : Bun.serve<BunSocketKey>({ unix: options.unix, ...handler });

    // accept an upgrade under a new key for its attachment
    const listener: Listener<Attachment> = {
        url: server.url,
        upgrade: (request, upgrade) => {
            // keep the attachment under the socket's key, dropping it on a refused upgrade
            const key = next;
            next += 1;
            attachments.set(key, { attachment: upgrade.attachment });
            const isUpgraded = server.upgrade(request, {
                data: { key },
                ...(upgrade.headers === undefined ? {} : { headers: upgrade.headers }),
            });
            if (!isUpgraded) {
                attachments.delete(key);
            }

            return isUpgraded;
        },
        stop: (isClosingActive) => server.stop(isClosingActive),
    };

    return listener;
}
