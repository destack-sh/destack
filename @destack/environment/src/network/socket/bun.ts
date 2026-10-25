import type * as bun from "bun";
import { present } from "@destack/schema";
import {
    type ConnectOptions,
    type HostAddress,
    refuseUnixTls,
    socketName,
    type SocketAddress,
    SocketInbox,
    type SocketListener,
    type SocketListenOptions,
    type Socket,
} from "./socket.ts";

/** One end of a connection over a Bun socket, read and written as web streams. */
class BunSocketEnd implements Socket {
    /** The bytes the peer sends. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The bytes sent to the peer. */
    readonly writable: WritableStream<Uint8Array>;
    /** Settles once the connection opened, or fails as it could not. */
    readonly opened: Promise<void>;
    /** Settles once the connection closed. */
    readonly closed: Promise<void>;
    /** The address and options it connected with, absent for an accepted end. */
    readonly #target:
        | { readonly address: SocketAddress; readonly options: ConnectOptions }
        | undefined;
    /** The settlers of the opening. */
    readonly #opening = Promise.withResolvers<void>();
    /** The settlers of the close. */
    readonly #closing = Promise.withResolvers<void>();
    /** The readable and its controller, taking received bytes. */
    readonly #inbox = SocketInbox.open(() => this.#socket?.terminate());
    /** The Bun socket, once it opened. */
    #socket: bun.Socket<BunSocketEnd> | undefined;
    /** The settlers of the next drain, while a write waits for the socket's buffer. */
    #drain: PromiseWithResolvers<void> | undefined;
    /** Whether the readable ended, by the peer's end, a failure or the TLS socket replacing it. */
    #isEnded = false;
    /** Whether a TLS socket replaced this end, which then refuses writes. */
    #isReplaced = false;

    /** Make an end not yet open, connected at an address or accepted by a listener. */
    constructor(connect?: { readonly address: SocketAddress; readonly options: ConnectOptions }) {
        // open with the socket, a failed connect failing only those who wait on it
        this.#target = connect;
        this.opened = this.#opening.promise;
        this.opened.catch(() => undefined);
        this.closed = this.#closing.promise;

        // read the received bytes, and write the sent ones until the socket's buffer fills
        this.readable = this.#inbox.readable;
        this.writable = new WritableStream<Uint8Array>({
            write: async (chunk) => {
                await this.opened;
                if (this.#isReplaced) {
                    throw new TypeError("the connection started TLS on another socket");
                }
                await this.#write(chunk);
            },
            close: async () => {
                await this.opened;
                this.#socket?.end();
            },
            abort: () => this.#socket?.terminate(),
        });
    }

    /** Take the Bun socket once it opened. */
    open(socket: bun.Socket<BunSocketEnd>): void {
        this.#socket = socket;
        this.#opening.resolve();
    }

    /** Take bytes the peer sent. */
    receive(chunk: Uint8Array): void {
        if (!this.#isEnded) {
            this.#inbox.controller.enqueue(chunk);
        }
    }

    /** Let a write waiting for the socket's buffer go on. */
    drain(): void {
        this.#drain?.resolve();
        this.#drain = undefined;
    }

    /** End the readable at the peer's end, or fail it and the opening with a failure. */
    end(failure?: Error): void {
        // end once, failing the opening of a connection that never opened
        if (this.#isEnded) {
            return;
        }
        this.#isEnded = true;
        this.#opening.reject(
            failure ??
                Object.assign(new TypeError("the connection closed before it opened"), {
                    code: "ConnectionClosed",
                }),
        );

        // close the readable, or fail it
        if (failure === undefined) {
            this.#inbox.controller.close();
        } else {
            this.#inbox.controller.error(failure);
        }
    }

    /** Fail a connect as the simulated network does, refused where nothing listens. */
    refuse(error: Error): void {
        // read a missing or unserved address as a refusal naming it
        const code = "code" in error ? error.code : undefined;
        const address = this.#target?.address;
        const isRefused = code === "ECONNREFUSED" || code === "ENOENT";
        this.settle(
            isRefused && address !== undefined
                ? Object.assign(
                      new TypeError(
                          `connection refused: nothing listens at ${socketName(address)}`,
                      ),
                      { code: "ConnectionRefused", cause: error },
                  )
                : error,
        );
    }

    /** Settle the close once the socket closed. */
    settle(failure?: Error): void {
        // end the readable, fail a write waiting for a drain, and settle the close
        this.end(failure);
        this.#drain?.reject(closed());
        this.#drain = undefined;
        this.#closing.resolve();
    }

    /** Close the connection. */
    async close(): Promise<void> {
        this.#socket?.end();
        await this.closed;
    }

    /** Let the process exit while the connection stays open, once it opened. */
    unref(): void {
        void this.opened.then(
            () => this.#socket?.unref(),
            () => undefined,
        );
    }

    /** Start TLS over a connection connected for it, answering the socket that replaces it. */
    startTls(): Socket {
        // refuse an upgrade of a connection not connected for one
        const socket = this.#socket;
        const target = this.#target;
        if (
            target?.options.secureTransport !== "starttls" ||
            !("hostname" in target.address) ||
            socket === undefined
        ) {
            throw new TypeError("only an open connection connected with starttls can start TLS");
        }

        // hand the socket to TLS, which reads and writes it from now on, retiring this end
        const secure = new BunSocketEnd();
        this.#isEnded = true;
        this.#isReplaced = true;
        this.#inbox.controller.close();
        this.#closing.resolve();
        socket.upgradeTLS({
            data: secure,
            tls: BunSocketEnd.tls(target.address, target.options),
            socket: HANDLERS,
        });

        return secure;
    }

    /** Read the TLS options of a connect: its server name and trusted authorities. */
    static tls(address: HostAddress, options: ConnectOptions): bun.TLSOptions {
        return {
            serverName: address.hostname,
            ...(options.certificateAuthorities === undefined
                ? {}
                : { ca: [...options.certificateAuthorities] }),
        };
    }

    /** Write a chunk, waiting for the socket's buffer to drain while it takes only a part. */
    async #write(chunk: Uint8Array): Promise<void> {
        let rest = chunk;
        while (rest.length > 0) {
            // write what the socket takes, and wait for a drain for the rest
            const written = present(this.#socket, "the open socket").write(rest);
            if (written < 0) {
                throw closed();
            }
            rest = rest.subarray(written);
            if (rest.length > 0) {
                this.#drain = Promise.withResolvers<void>();
                await this.#drain.promise;
            }
        }
    }
}

/** The handlers of every Bun socket, passing each event to the end the socket keeps. */
const HANDLERS: bun.SocketHandler<BunSocketEnd> = {
    open: (socket) => socket.data.open(socket),
    data: (socket, chunk) => socket.data.receive(chunk),
    drain: (socket) => socket.data.drain(),
    end: (socket) => socket.data.end(),
    error: (socket, error) => socket.data.end(error),
    connectError: (socket, error) => socket.data.refuse(error),
    close: (socket, error) => socket.data.settle(error),
};

/** Connect through Bun's sockets to a host and port, in TLS when asked, or to a Unix socket. */
export function connectSocket(address: SocketAddress, options: ConnectOptions = {}): Socket {
    // refuse runtimes without Bun's sockets, and TLS toward a Unix socket
    if (typeof Bun === "undefined") {
        throw new TypeError("connecting requires the Bun runtime");
    }
    refuseUnixTls(address, options);

    // connect to the Unix socket, or to the host in plain text or in TLS
    const end = new BunSocketEnd({ address, options });
    const connecting =
        "unix" in address
            ? Bun.connect<BunSocketEnd>({ unix: address.unix, data: end, socket: HANDLERS })
            : Bun.connect<BunSocketEnd>({
                  hostname: address.hostname,
                  port: address.port,
                  data: end,
                  socket: HANDLERS,
                  ...(options.secureTransport === "on"
                      ? { tls: BunSocketEnd.tls(address, options) }
                      : {}),
              });
    void connecting.catch(() => undefined);

    return end;
}

/** Listen for plain connections through Bun's sockets, at a host and port or a Unix socket. */
export async function listenSocket(options: SocketListenOptions): Promise<SocketListener> {
    // refuse runtimes without Bun's sockets
    if (typeof Bun === "undefined") {
        throw new TypeError("listening requires the Bun runtime");
    }

    // accept each connection as an end of its own
    const socket: bun.SocketHandler<BunSocketEnd> = {
        ...HANDLERS,
        open: (accepted) => {
            accepted.data = new BunSocketEnd();
            accepted.data.open(accepted);
            options.accept(accepted.data);
        },
    };

    // listen at the Unix socket, or at the host and the given or a free port
    if (options.unix !== undefined) {
        const listener = Bun.listen<BunSocketEnd>({ unix: options.unix, socket });

        return {
            address: { unix: options.unix },
            stop: async () => listener.stop(true),
            unref: () => listener.unref(),
        };
    }
    const hostname = options.hostname ?? "localhost";
    const listener = Bun.listen<BunSocketEnd>({ hostname, port: options.port ?? 0, socket });

    return {
        address: { hostname, port: listener.port },
        stop: async () => listener.stop(true),
        unref: () => listener.unref(),
    };
}

/** Describe a write to a connection that closed, as the simulated network reports it. */
function closed(): TypeError {
    return Object.assign(new TypeError("the connection closed"), { code: "ConnectionClosed" });
}
