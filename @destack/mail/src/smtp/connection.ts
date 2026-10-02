import net, { type Socket } from "node:net";
import tls from "node:tls";
import type { Security } from "./client.ts";
import { SmtpError } from "./error.ts";
import { MAX_REPLY_LINE_OCTETS, Reply, type ReplyLine } from "./reply.ts";

/** Where a connection reaches its server, and how long it waits on it. */
export interface ConnectionOptions {
    /** The server's host name or IP address. */
    readonly host: string;
    /** The server's port. */
    readonly port: number;
    /** How the connection protects its bytes. */
    readonly security: Security;
    /** How long the connection waits on a silent server, in milliseconds. */
    readonly timeout: number;
    /** How long one reply, handshake or token may take in all, in milliseconds. */
    readonly deadline: number;
    /** The PEM certificates trusted as authorities instead of the system's. */
    readonly certificateAuthorities: readonly string[] | undefined;
}

/** One SMTP session's transport: a plain or TLS socket and the replies read from it. */
export class SmtpConnection {
    /** Where the connection reaches its server, and how long it waits. */
    readonly #options: ConnectionOptions;
    /** The socket carrying the session, a TLS socket after an upgrade. */
    #socket: Socket;
    /** The received bytes not yet split into lines. */
    #received: Buffer = Buffer.alloc(0);
    /** The lines of the reply being received. */
    #lines: ReplyLine[] = [];
    /** The replies received and not yet read, in order. */
    readonly #replies: Reply[] = [];
    /** The failure that ended the connection. */
    #failure: SmtpError | undefined;
    /** The read waiting for the next reply. */
    #waiter: ((reply: Reply) => void) | undefined;
    /** The rejection of the step waiting under the deadline. */
    #abort: ((error: SmtpError) => void) | undefined;

    /** Wrap a socket and read replies from it. */
    private constructor(socket: Socket, options: ConnectionOptions) {
        this.#options = options;
        this.#socket = socket;
        this.#attach(socket);
    }

    /** Connect to the server, completing the TLS handshake first under implicit TLS. */
    static async open(options: ConnectionOptions): Promise<SmtpConnection> {
        // connect in plain text or in TLS from the first byte
        const socket =
            options.security === "tls"
                ? tls.connect({ port: options.port, ...secureOptions(options) })
                : net.connect({ host: options.host, port: options.port });
        const connection = new SmtpConnection(socket, options);
        await connection.#reach(socket, options.security === "tls" ? "secureConnect" : "connect");

        return connection;
    }

    /** Read the server's next reply. */
    read(): Promise<Reply> {
        // take a reply already received, or wait for the next one
        const reply = this.#replies.shift();
        if (reply !== undefined) {
            return Promise.resolve(reply);
        } else if (this.#failure !== undefined) {
            return Promise.reject(this.#failure);
        } else {
            const { promise, resolve } = Promise.withResolvers<Reply>();
            this.#waiter = resolve;

            return this.race(promise);
        }
    }

    /** Send one command line and read its reply. */
    command(line: string): Promise<Reply> {
        this.write(`${line}\r\n`);

        return this.read();
    }

    /** Write raw text to the server. */
    write(text: string): void {
        this.#socket.write(text);
    }

    /** Settle with a promise, or fail on the connection's failure or its deadline first. */
    race<Value>(promise: Promise<Value>): Promise<Value> {
        // fail at once on a failed connection
        const failure = this.#failure;
        if (failure !== undefined) {
            return Promise.reject(failure);
        }

        // arm the deadline, which fails the whole connection
        const { host, port, deadline } = this.#options;
        const timer = setTimeout(() => {
            this.#fail(
                new SmtpError("TIMEOUT", `${host}:${port} missed the ${deadline} ms deadline`),
            );
        }, deadline);

        // settle with the promise or the failure, whichever comes first
        const raced = new Promise<Value>((resolve, reject) => {
            this.#abort = reject;
            promise.then(resolve, reject);
        });

        return raced.finally(() => {
            clearTimeout(timer);
            this.#abort = undefined;
        });
    }

    /** Upgrade the connection to TLS after the server's STARTTLS reply, as RFC 3207 describes. */
    async upgrade(): Promise<void> {
        // refuse text the server sent ahead of the handshake
        if (this.#received.length > 0 || this.#lines.length > 0 || this.#replies.length > 0) {
            const error = new SmtpError("PROTOCOL", "server sent data after its STARTTLS reply");
            this.#fail(error);
            throw error;
        }

        // hand the plain socket to TLS, which reads it from now on
        const plain = this.#socket;
        this.#detach(plain);
        const secure = tls.connect({ socket: plain, ...secureOptions(this.#options) });
        this.#socket = secure;
        this.#attach(secure);
        await this.#reach(secure, "secureConnect");
    }

    /** Send QUIT and wait for the server to end the connection, closing it at the deadline at the latest. */
    async close(): Promise<void> {
        // ask the server to end the session, sending nothing more
        const closed = new Promise<void>((resolve) => {
            this.#socket.once("close", () => resolve());
        });
        this.write("QUIT\r\n");
        this.#socket.end();

        // wait for the socket to close, destroying it at the deadline
        const timer = setTimeout(() => this.#socket.destroy(), this.#options.deadline);
        await closed;
        clearTimeout(timer);
    }

    /** Close the connection at once. */
    destroy(): void {
        this.#socket.destroy();
    }

    /** Listen to a socket's data, failures and silence. */
    #attach(socket: Socket): void {
        // arm the idle timeout and route the socket's events to the connection
        socket.setTimeout(this.#options.timeout);
        socket.on("data", this.#receive);
        socket.on("error", this.#error);
        socket.on("close", this.#close);
        socket.on("timeout", this.#timeout);
    }

    /** Stop listening to a socket. */
    #detach(socket: Socket): void {
        // disarm the idle timeout and drop the socket's routes
        socket.setTimeout(0);
        socket.off("data", this.#receive);
        socket.off("error", this.#error);
        socket.off("close", this.#close);
        socket.off("timeout", this.#timeout);
    }

    /** Wait for a socket to connect or complete its handshake, or for the connection's failure. */
    async #reach(socket: Socket, event: "connect" | "secureConnect"): Promise<void> {
        // wait for the event, removing its listener however the wait ends
        const { promise, resolve } = Promise.withResolvers<void>();
        socket.once(event, resolve);
        try {
            await this.race(promise);
        } finally {
            socket.off(event, resolve);
        }
    }

    /** Split received bytes into reply lines, and lines into replies. */
    readonly #receive = (chunk: Buffer): void => {
        // take each complete line
        this.#received = Buffer.concat([this.#received, chunk]);
        let end = this.#received.indexOf("\r\n");
        while (end !== -1 && this.#failure === undefined) {
            const line = this.#received.subarray(0, end);
            this.#received = this.#received.subarray(end + 2);
            this.#receiveLine(line);
            end = this.#received.indexOf("\r\n");
        }

        // refuse a partial line that cannot end within the limit
        if (this.#received.length >= MAX_REPLY_LINE_OCTETS) {
            this.#fail(
                new SmtpError(
                    "PROTOCOL",
                    `server sent a reply line over ${MAX_REPLY_LINE_OCTETS} octets`,
                ),
            );
        }
    };

    /** Add one line to the reply being received, completing the reply on its last line. */
    #receiveLine(bytes: Uint8Array): void {
        // parse the line, failing the connection on another shape
        let parsed: ReplyLine;
        try {
            parsed = Reply.parseLine(bytes);
        } catch (error) {
            if (!(error instanceof SmtpError)) {
                throw error;
            }
            this.#fail(error);
            return;
        }

        // require every line of a reply to carry the same code
        const first = this.#lines[0];
        if (first !== undefined && first.code !== parsed.code) {
            this.#fail(
                new SmtpError(
                    "PROTOCOL",
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

    /** Settle the waiting read with a reply, or keep the reply for the next read. */
    #deliver(reply: Reply): void {
        const waiter = this.#waiter;
        this.#waiter = undefined;
        if (waiter === undefined) {
            this.#replies.push(reply);
        } else {
            waiter(reply);
        }
    }

    /** Fail the connection on a socket error, such as a refused connection or handshake. */
    readonly #error = (error: Error): void => {
        const { host, port } = this.#options;
        const message = `connection to ${host}:${port} failed: ${error.message}`;
        this.#fail(new SmtpError("CONNECTION", message, undefined, { cause: error }));
    };

    /** Fail the connection when the server closes it. */
    readonly #close = (): void => {
        const { host, port } = this.#options;
        this.#fail(new SmtpError("CONNECTION", `connection to ${host}:${port} closed`));
    };

    /** Fail the connection when the server stays silent for the timeout. */
    readonly #timeout = (): void => {
        const { host, port, timeout } = this.#options;
        this.#fail(new SmtpError("TIMEOUT", `${host}:${port} sent nothing for ${timeout} ms`));
    };

    /** End the connection with its first failure, settling the waiting step with it. */
    #fail(error: SmtpError): void {
        // keep the first failure
        if (this.#failure !== undefined) {
            return;
        }
        this.#failure = error;

        // reject the waiting step and close the socket
        this.#waiter = undefined;
        this.#abort?.(error);
        this.#socket.destroy();
    }
}

/** Build the TLS options that verify the server's certificate for its host. */
function secureOptions(options: ConnectionOptions): tls.ConnectionOptions {
    // send the host as SNI unless it is an IP address
    return {
        host: options.host,
        ...(net.isIP(options.host) === 0 ? { servername: options.host } : {}),
        ...(options.certificateAuthorities === undefined
            ? {}
            : { ca: [...options.certificateAuthorities] }),
    };
}
