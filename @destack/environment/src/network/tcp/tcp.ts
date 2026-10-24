import { present } from "@destack/schema";

/** A TCP endpoint: a host name or IP address and a port. */
export interface SocketAddress {
    /** The host name or IP address. */
    readonly hostname: string;
    /** The port. */
    readonly port: number;
}

/** How a TCP connection protects its bytes: plain, in TLS, or plain until it starts TLS. */
export type SecureTransport = "off" | "on" | "starttls";

/** How a TCP connection opens. */
export interface ConnectOptions {
    /** How it protects its bytes, plain by default. */
    readonly secureTransport?: SecureTransport;
    /** The PEM certificates trusted as authorities instead of the system's. */
    readonly certificateAuthorities?: readonly string[];
}

/** A TCP connection's byte streams, as the sockets of Cloudflare's `connect()` have them. */
export interface TcpSocket {
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
    readonly startTls: () => TcpSocket;
}

/** What a TCP listener accepts. */
export interface TcpListenOptions {
    /** The host name it listens at, localhost by default. */
    readonly hostname?: string;
    /** The port it listens at, a free one for zero or none. */
    readonly port?: number;
    /** Take an accepted connection. */
    readonly accept: (socket: TcpSocket) => void;
}

/** An open TCP listener. */
export interface TcpListener {
    /** The address it listens at. */
    readonly address: SocketAddress;
    /** Stop listening, closing the connections it accepted. */
    readonly stop: () => Promise<void>;
}

/** The readable of a TCP end, with the controller its connection delivers through. */
export class TcpInbox {
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
    static open(cancel: () => void): TcpInbox {
        let started: ReadableStreamDefaultController<Uint8Array> | undefined;
        const readable = new ReadableStream<Uint8Array>({
            start: (controller) => {
                started = controller;
            },
            cancel,
        });

        return new TcpInbox(readable, present(started, "the inbox's controller"));
    }
}
