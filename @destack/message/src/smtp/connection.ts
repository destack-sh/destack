import type { Environment, SecureTransport, Socket } from "@destack/environment";
import type { FamilyFailure } from "@destack/error";
import { MessageError } from "../error/error.ts";
import { MAX_REPLY_LINE_OCTETS, Reply, type ReplyLine } from "./reply.ts";

/** The line ending of SMTP commands and replies (RFC 5321 2.3.8). */
const CRLF = new Uint8Array([13, 10]);

/** The server a connection opens, how long it waits on it, and the network and clock it runs on. */
export interface ConnectionOptions {
    /** The server's host name or IP address. */
    readonly host: string;
    /** The server's port. */
    readonly port: number;
    /** How the connection protects its bytes. */
    readonly secureTransport: SecureTransport;
    /** How long the dial and the TLS handshake may take, in milliseconds. */
    readonly timeout: number;
    /** The network the connection dials and the clock its timers run on. */
    readonly environment: Environment;
}

/** One SMTP session's transport: a TCP socket, plain or in TLS, and the replies read from it. */
export class SmtpConnection {
    /** The server the connection opens, and how long it waits. */
    readonly #options: ConnectionOptions;
    /** The socket of the session, the TLS one after an upgrade. */
    #socket: Socket;
    /** The writer of the socket's bytes. */
    #writer: WritableStreamDefaultWriter<Uint8Array>;
    /** The reader of the socket's bytes. */
    #reader: ReadableStreamDefaultReader<Uint8Array>;
    /** The received bytes not yet split into lines. */
    #received: Uint8Array = new Uint8Array();
    /** The lines of the reply being received. */
    #lines: ReplyLine[] = [];
    /** The replies received and not yet read, in order. */
    readonly #replies: Reply[] = [];
    /** The failure that ended the connection. */
    #failure: FamilyFailure | undefined;
    /** The read waiting for the next reply. */
    #waiter: ((reply: Reply) => void) | undefined;
    /** The rejection of the step waiting under its timeout. */
    #abort: ((error: FamilyFailure) => void) | undefined;

    /** Wrap a socket and read replies from it. */
    private constructor(socket: Socket, options: ConnectionOptions) {
        // take the options and lock the socket's streams
        this.#options = options;
        this.#socket = socket;
        this.#writer = socket.writable.getWriter();
        this.#reader = socket.readable.getReader();
    }

    /** Connect to the server, in TLS from the first byte under implicit TLS. */
    static async open(options: ConnectionOptions): Promise<SmtpConnection> {
        // dial per secure transport
        const { host, port, secureTransport, environment } = options;
        const socket = environment.network.connect({ hostname: host, port }, { secureTransport });

        // wait for it to open
        const connection = new SmtpConnection(socket, options);
        await connection.#opening(socket);
        connection.#listen();

        return connection;
    }

    /** Read the server's next reply within a timeout. */
    read(timeout: number): Promise<Reply> {
        // take a reply already received, or wait for the next one
        const reply = this.#replies.shift();
        if (reply !== undefined) {
            return Promise.resolve(reply);
        } else if (this.#failure !== undefined) {
            return Promise.reject(this.#failure);
        } else {
            const { promise, resolve } = Promise.withResolvers<Reply>();
            this.#waiter = resolve;

            return this.race(promise, timeout);
        }
    }

    /** Send one command line and read its reply within a timeout. */
    command(line: string, timeout: number): Promise<Reply> {
        void this.write(new TextEncoder().encode(`${line}\r\n`));

        return this.read(timeout);
    }

    /** Write raw bytes to the server, failing the connection if the write fails. */
    write(bytes: Uint8Array): Promise<void> {
        return this.#writer.write(bytes).catch((error: unknown) => {
            this.#broken(error);
        });
    }

    /** Settle with a promise, or fail on the connection's failure or the timeout first. */
    race<Value>(promise: Promise<Value>, timeout: number): Promise<Value> {
        // fail at once on a failed connection
        const failure = this.#failure;
        if (failure !== undefined) {
            return Promise.reject(failure);
        }

        // arm the timeout on the clock, which fails the whole connection
        const { host, port, environment } = this.#options;
        const disarm = new AbortController();
        environment.clock.sleep(timeout, disarm.signal).then(
            () =>
                this.#fail(
                    MessageError.of(
                        "PROVIDER_TIMEOUT",
                        `${host}:${port} timed out after ${timeout} ms`,
                    ),
                ),
            () => undefined,
        );

        // settle with the promise or the failure, whichever comes first
        const raced = new Promise<Value>((resolve, reject) => {
            this.#abort = reject;
            promise.then(resolve, reject);
        });

        return raced.finally(() => {
            disarm.abort();
            this.#abort = undefined;
        });
    }

    /** Upgrade the connection to TLS after the server's STARTTLS reply, as RFC 3207 describes. */
    async upgrade(): Promise<void> {
        // refuse text the server sent ahead of the handshake
        if (this.#received.length > 0 || this.#lines.length > 0 || this.#replies.length > 0) {
            const error = MessageError.of(
                "RESPONSE_INVALID",
                "server sent data after its STARTTLS reply",
            );
            this.#fail(error);
            throw error;
        }

        // stop reading the plain socket and hand it to TLS, which reads it from now on
        const plain = this.#socket;
        this.#reader.releaseLock();
        this.#writer.releaseLock();
        const secure = plain.startTls();
        this.#socket = secure;
        this.#writer = secure.writable.getWriter();
        this.#reader = secure.readable.getReader();
        await this.#opening(secure);
        this.#listen();
    }

    /** Send QUIT and wait for the server to end the connection, closing it at the timeout. */
    async close(timeout: number): Promise<void> {
        // ask the server to end the session, sending nothing more
        void this.write(new TextEncoder().encode("QUIT\r\n"));
        await this.#writer.close().catch(() => undefined);

        // wait for the connection to close, closing it at the timeout
        const disarm = new AbortController();
        const expired = this.#options.environment.clock.sleep(timeout, disarm.signal).then(
            () => this.#socket.close(),
            () => undefined,
        );
        await this.#socket.closed;
        disarm.abort();
        await expired;
    }

    /** Close the connection at once. */
    destroy(): void {
        void this.#socket.close().catch(() => undefined);
    }

    /** Wait for a socket to open or complete its handshake, or for the connection's failure. */
    async #opening(socket: Socket): Promise<void> {
        // read a failed dial or handshake as an unreachable server
        const { host, port } = this.#options;
        const opened = socket.opened.catch((error: unknown) => {
            // fail the connection with the dial's or handshake's reason
            const reason = error instanceof Error ? error.message : String(error);
            const failure = MessageError.of(
                "PROVIDER_UNREACHABLE",
                `connection to ${host}:${port} failed: ${reason}`,
                {
                    cause: error,
                },
            );
            this.#fail(failure);
            throw failure;
        });

        await this.race(opened, this.#options.timeout);
    }

    /** Read the socket's bytes until it ends, failing the connection on its end or its silence. */
    #listen(): void {
        // read each chunk until the server ends the connection
        const reader = this.#reader;
        const reading = async () => {
            for (;;) {
                const { done, value } = await reader.read();
                if (done) {
                    this.#closed();

                    return;
                }
                this.#receive(value);
            }
        };

        // fail on a broken connection, leaving a reader the upgrade released
        reading().catch((error: unknown) => {
            if (reader === this.#reader) {
                this.#broken(error);
            }
        });
    }

    /** Split received bytes into reply lines, and lines into replies. */
    #receive(chunk: Uint8Array): void {
        // take each complete line
        const joined = new Uint8Array(this.#received.length + chunk.length);
        joined.set(this.#received);
        joined.set(chunk, this.#received.length);
        this.#received = joined;
        let end = indexOfLineEnd(this.#received);
        while (end !== -1 && this.#failure === undefined) {
            const line = this.#received.subarray(0, end);
            this.#received = this.#received.subarray(end + CRLF.length);
            this.#receiveLine(line);
            end = indexOfLineEnd(this.#received);
        }

        // refuse a partial line that cannot end within the limit
        if (this.#received.length >= MAX_REPLY_LINE_OCTETS) {
            this.#fail(
                MessageError.of(
                    "RESPONSE_INVALID",
                    `server sent a reply line longer than ${MAX_REPLY_LINE_OCTETS} octets`,
                ),
            );
        }
    }

    /** Add one line to the reply being received, completing the reply on its last line. */
    #receiveLine(bytes: Uint8Array): void {
        // parse the line, failing the connection on another shape
        let parsed: ReplyLine;
        try {
            parsed = Reply.parseLine(bytes);
        } catch (error) {
            if (!MessageError.is(error)) {
                throw error;
            }
            this.#fail(error);

            return;
        }

        // require every line of a reply to have the same code
        const first = this.#lines[0];
        if (first !== undefined && first.code !== parsed.code) {
            this.#fail(
                MessageError.of(
                    "RESPONSE_INVALID",
                    `server continued a ${first.code} reply with a ${parsed.code} line`,
                ),
            );

            return;
        }

        // complete the reply on its last line
        this.#lines.push(parsed);
        if (parsed.isLast) {
            const reply = Reply.join(this.#lines);
            this.#lines = [];
            this.#deliver(reply);
        }
    }

    /** Settle the waiting read with a reply, or store the reply for the next read. */
    #deliver(reply: Reply): void {
        // settle the read or keep the reply
        const waiter = this.#waiter;
        this.#waiter = undefined;
        if (waiter === undefined) {
            this.#replies.push(reply);
        } else {
            waiter(reply);
        }
    }

    /** Fail the connection on a broken socket. */
    #broken(error: unknown): void {
        // fail with the socket's reason
        const { host, port } = this.#options;
        const reason = error instanceof Error ? error.message : String(error);
        const message = `connection to ${host}:${port} failed: ${reason}`;
        this.#fail(MessageError.of("PROVIDER_UNREACHABLE", message, { cause: error }));
    }

    /** Fail the connection when the server ends it. */
    #closed(): void {
        const { host, port } = this.#options;
        this.#fail(MessageError.of("PROVIDER_UNREACHABLE", `connection to ${host}:${port} closed`));
    }

    /** End the connection with its first failure, settling the waiting step with it. */
    #fail(error: FamilyFailure): void {
        // record the first failure
        if (this.#failure !== undefined) {
            return;
        }
        this.#failure = error;

        // reject the waiting step and close the socket
        this.#waiter = undefined;
        this.#abort?.(error);
        this.destroy();
    }
}

/** Find the first CRLF in some bytes, -1 without one. */
function indexOfLineEnd(bytes: Uint8Array): number {
    for (let index = 0; index + 1 < bytes.length; index++) {
        if (bytes[index] === CRLF[0] && bytes[index + 1] === CRLF[1]) {
            return index;
        }
    }

    return -1;
}
