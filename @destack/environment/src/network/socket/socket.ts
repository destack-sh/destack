import { present } from "@destack/schema";

/** A TCP endpoint: a host name or IP address and a port. */
export interface HostAddress {
    /** The host name or IP address. */
    readonly hostname: string;
    /** The port. */
    readonly port: number;
}

/** A Unix socket's endpoint: its path on this computer, as Bun's `unix` option has it. */
export interface UnixAddress {
    /** The socket's path. */
    readonly unix: string;
}

/** Where a stream socket connects or listens: a host and port over TCP, or a Unix socket. */
export type SocketAddress = HostAddress | UnixAddress;

/** How a connection protects its bytes: plain, in TLS, or plain until it starts TLS. */
export type SecureTransport = "off" | "on" | "starttls";

/** How a connection opens. */
export interface ConnectOptions {
    /** How it protects its bytes, plain by default and always for a Unix socket. */
    readonly secureTransport?: SecureTransport;
    /** The PEM certificates trusted as authorities instead of the system's. */
    readonly certificateAuthorities?: readonly string[];
}

/** A connection's byte streams, as the sockets of Cloudflare's `connect()` have them. */
export interface Socket {
    /** The bytes the peer sends. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The bytes sent to the peer. */
    readonly writable: WritableStream<Uint8Array>;
    /** Settles once the connection opened, or fails as it could not. */
    readonly opened: Promise<void>;
    /** Settles once the connection closed. */
    readonly closed: Promise<void>;
    /** Close the connection. */
    readonly close: () => Promise<void>;
    /** Start TLS over a connection connected for it, answering the socket that replaces it. */
    readonly startTls: () => Socket;
    /** Let the process exit while the connection stays open, as Deno's `Conn.unref` does. */
    readonly unref: () => void;
}

/** Where a listener listens and what it accepts, as `Bun.listen` takes it. */
export type SocketListenOptions = (
    | {
          /** The host name it listens at, localhost by default. */
          readonly hostname?: string;
          /** The port it listens at, a free one for zero or none. */
          readonly port?: number;
          /** No Unix socket. */
          readonly unix?: never;
      }
    | {
          /** The Unix socket's path it listens at. */
          readonly unix: string;
          /** No host name. */
          readonly hostname?: never;
          /** No port. */
          readonly port?: never;
      }
) & {
    /** Take an accepted connection. */
    readonly accept: (socket: Socket) => void;
};

/** An open listener of stream sockets. */
export interface SocketListener {
    /** The address it listens at. */
    readonly address: SocketAddress;
    /** Stop listening, closing the connections it accepted. */
    readonly stop: () => Promise<void>;
    /** Let the process exit while it listens, as Deno's `Listener.unref` does. */
    readonly unref: () => void;
}

/** The readable of a socket end, with the controller its connection delivers through. */
export class SocketInbox {
    /** The bytes the end reads. */
    readonly readable: ReadableStream<Uint8Array>;
    /** The controller taking delivered bytes. */
    readonly controller: ReadableStreamDefaultController<Uint8Array>;

    /** Keep a readable and its controller. */
    private constructor(
        readable: ReadableStream<Uint8Array>,
        controller: ReadableStreamDefaultController<Uint8Array>,
    ) {
        this.readable = readable;
        this.controller = controller;
    }

    /** Open an empty inbox, calling back once its reader cancels. */
    static open(cancel: () => void): SocketInbox {
        let started: ReadableStreamDefaultController<Uint8Array> | undefined;
        const readable = new ReadableStream<Uint8Array>({
            start: (controller) => {
                started = controller;
            },
            cancel,
        });

        return new SocketInbox(readable, present(started, "the inbox's controller"));
    }
}

/** Write an address as people read it: its host and port, or the Unix socket's path. */
export function socketName(address: SocketAddress): string {
    return "unix" in address ? address.unix : `${address.hostname}:${address.port}`;
}

/** Refuse TLS toward a Unix socket, which has no server name to verify. */
export function refuseUnixTls(address: SocketAddress, options: ConnectOptions): void {
    if ("unix" in address && (options.secureTransport ?? "off") !== "off") {
        throw new TypeError("a Unix socket connection cannot use TLS");
    }
}
