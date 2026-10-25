import type { Server, SocketListener, Socket } from "@destack/environment";
import { nth } from "@destack/schema";
import type { SecureTransport } from "@destack/environment";
import type { BearerCredentials, PlainCredentials } from "../smtp/index.ts";

/** The XOAUTH2 error challenge: base64 of the JSON status a mail provider sends before 535. */
const XOAUTH2_CHALLENGE = btoa('{"status":"401","schemes":"bearer"}');

/** The loopback address test mail servers listen at, where plain text is allowed. */
const LOOPBACK = "127.0.0.1";

/** How a test mail server's sessions run: their protection, credentials and replies. */
export interface TestMailServerOptions {
    /** How sessions protect their bytes; TLS is nothing on a simulated network. */
    readonly secureTransport: SecureTransport;
    /** The PLAIN credentials accepted. */
    readonly plain?: PlainCredentials;
    /** The XOAUTH2 credentials accepted. */
    readonly xoauth2?: BearerCredentials<string>;
    /** The scripted replies by client line: `greeting` for the banner, `.` for the data's end. */
    readonly answers?: Readonly<Record<string, string>>;
    /** The client lines the server never answers, such as `.` to stall the data's end. */
    readonly silences?: readonly string[];
}

/** An SMTP server on a network's loopback that records every session and answers as scripted. */
export class TestMailServer implements AsyncDisposable {
    /** Each session's lines: `C:` from the client, `S:` from the server, `TLS` on a handshake. */
    readonly transcripts: string[][] = [];
    /** The content of each DATA command in order, without dot-stuffing and the ending dot. */
    readonly messages: string[] = [];
    /** How sessions run. */
    readonly #options: TestMailServerOptions;
    /** The listener accepting sessions, absent until it listens. */
    #listener: SocketListener | undefined;

    /** Create a server that serves each connection. */
    private constructor(options: TestMailServerOptions) {
        this.#options = options;
    }

    /** Listen on a free port of a server's loopback. */
    static async listen(server: Server, options: TestMailServerOptions): Promise<TestMailServer> {
        const mail = new TestMailServer(options);
        mail.#listener = await server.socket({
            hostname: LOOPBACK,
            accept: (socket) => mail.#accept(socket),
        });

        return mail;
    }

    /** The port the server listens on. */
    get port(): number {
        const address = this.#listener?.address;
        if (address === undefined) {
            throw new TypeError("the test mail server is not listening");
        } else if ("unix" in address) {
            throw new TypeError("the test mail server listens at a Unix socket, not a port");
        }

        return address.port;
    }

    /** Stop listening, closing every session. */
    async close(): Promise<void> {
        await this.#listener?.stop();
    }

    /** Close the server. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Start a session on a new connection, in TLS from the first byte under implicit TLS. */
    #accept(socket: Socket): void {
        // start the transcript
        const isSecure = this.#options.secureTransport === "on";
        const transcript: string[] = isSecure ? ["TLS"] : [];
        this.transcripts.push(transcript);

        // greet and serve
        const session = new Session(this.#options, transcript, this.messages, socket, isSecure);
        session.reply("greeting", "220 mail.test ESMTP ready");
    }
}

/** One session's state: its socket, its received text and the command it waits on. */
class Session {
    /** How the session runs. */
    readonly #options: TestMailServerOptions;
    /** The session's transcript. */
    readonly #transcript: string[];
    /** The server's received messages. */
    readonly #messages: string[];
    /** The socket, which carries the TLS session after STARTTLS. */
    readonly #socket: Socket;
    /** The writer of the socket's bytes. */
    #writer: WritableStreamDefaultWriter<Uint8Array>;
    /** The reader of the socket's bytes, the TLS one's after STARTTLS. */
    #reader: ReadableStreamDefaultReader<Uint8Array>;
    /** Whether TLS protects the session. */
    #isSecure: boolean;
    /** The received text not yet split into lines. */
    #received = "";
    /** The data lines of the message being received, absent outside DATA. */
    #data: string[] | undefined;
    /** Whether the session waits on the empty line after an XOAUTH2 challenge. */
    #isChallenged = false;

    /** Serve a socket. */
    constructor(
        options: TestMailServerOptions,
        transcript: string[],
        messages: string[],
        socket: Socket,
        isSecure: boolean,
    ) {
        // store the session's state and read its socket
        this.#options = options;
        this.#transcript = transcript;
        this.#messages = messages;
        this.#socket = socket;
        this.#writer = socket.writable.getWriter();
        this.#isSecure = isSecure;
        this.#reader = socket.readable.getReader();
        void this.#listen(this.#reader);
    }

    /** Read a socket's bytes until it ends, the upgrade releases it, or it breaks. */
    async #listen(reader: ReadableStreamDefaultReader<Uint8Array>): Promise<void> {
        const decoder = new TextDecoder();
        try {
            for (;;) {
                const { done, value } = await reader.read();
                if (done) {
                    return;
                }
                this.#read(decoder.decode(value, { stream: true }), reader);
            }
        } catch {
            return;
        }
    }

    /** Write the scripted or standard reply to a client line, unless the line gets silence. */
    reply(line: string, standard: string): string {
        // send nothing to a line the server stays silent on
        if (this.#options.silences?.includes(line) === true) {
            return "";
        }

        // write the reply's lines in one write, as a server sends a reply
        const reply = this.#options.answers?.[line] ?? standard;
        const lines = reply.split("\n");
        for (const replyLine of lines) {
            this.#transcript.push(`S: ${replyLine}`);
        }
        const text = lines.map((replyLine) => `${replyLine}\r\n`).join("");
        void this.#writer.write(new TextEncoder().encode(text)).catch(() => undefined);

        return reply;
    }

    /** Split received text into lines and handle each, until the upgrade replaces the reader. */
    #read(text: string, reader: ReadableStreamDefaultReader<Uint8Array>): void {
        this.#received += text;
        let end = this.#received.indexOf("\r\n");
        while (end !== -1 && !this.#isReleased(reader)) {
            const line = this.#received.slice(0, end);
            this.#received = this.#received.slice(end + 2);
            this.#handle(line, reader);
            end = this.#received.indexOf("\r\n");
        }
    }

    /** Report whether the upgrade released a reader. */
    #isReleased(reader: ReadableStreamDefaultReader<Uint8Array>): boolean {
        return this.#reader !== reader;
    }

    /** Handle one client line: a data line, the data's end or a command. */
    #handle(line: string, reader: ReadableStreamDefaultReader<Uint8Array>): void {
        // collect data lines until the lone dot, removing the dot-stuffing
        if (this.#data !== undefined && line !== ".") {
            this.#data.push(line.startsWith(".") ? line.slice(1) : line);
        }
        // end the data
        else if (this.#data !== undefined) {
            this.#transcript.push(`C: ${line}`);
            this.#messages.push(this.#data.map((dataLine) => `${dataLine}\r\n`).join(""));
            this.#data = undefined;
            this.reply(line, "250 2.0.0 queued");
        }
        // answer a command
        else {
            this.#transcript.push(`C: ${line}`);
            this.#command(line, reader);
        }
    }

    /** Answer one command line, or the empty line after an XOAUTH2 challenge. */
    #command(line: string, reader: ReadableStreamDefaultReader<Uint8Array>): void {
        // split the verb from its argument
        const verb = nth(line.split(" "), 0).toUpperCase();
        const argument = line.slice(verb.length + 1);

        // fail the XOAUTH2 exchange after the client's empty answer
        if (this.#isChallenged) {
            this.#isChallenged = false;
            this.reply(line, "535 5.7.8 authentication failed");
        }
        // list the extensions, offering STARTTLS until TLS protects the session
        else if (verb === "EHLO") {
            const isOffered = this.#options.secureTransport === "starttls" && !this.#isSecure;
            const extensions = [
                `250-mail.test greets ${argument}`,
                "250-AUTH PLAIN XOAUTH2",
                ...(isOffered ? ["250-STARTTLS"] : []),
                "250 ENHANCEDSTATUSCODES",
            ];
            this.reply(line, extensions.join("\n"));
        }
        // upgrade to TLS after a 220
        else if (verb === "STARTTLS") {
            if (this.reply(line, "220 2.0.0 ready to start TLS").startsWith("220")) {
                this.#upgrade(reader);
            }
        }
        // authenticate
        else if (verb === "AUTH") {
            this.#authenticate(line, argument);
        }
        // accept the sender and each recipient
        else if (verb === "MAIL") {
            this.reply(line, "250 2.1.0 sender ok");
        } else if (verb === "RCPT") {
            this.reply(line, "250 2.1.5 recipient ok");
        }
        // start collecting data after a 354
        else if (verb === "DATA") {
            if (this.reply(line, "354 end data with <CR><LF>.<CR><LF>").startsWith("354")) {
                this.#data = [];
            }
        }
        // close the session
        else if (verb === "QUIT") {
            this.reply(line, "221 2.0.0 bye");
            void this.#writer.close().catch(() => undefined);
        }
        // refuse anything else
        else {
            this.reply(line, "502 5.5.2 command not recognised");
        }
    }

    /** Answer AUTH PLAIN and AUTH XOAUTH2 against the accepted credentials. */
    #authenticate(line: string, argument: string): void {
        // decode the mechanism's initial response
        const [mechanism, response] = argument.split(" ");
        const decoded = new TextDecoder().decode(Uint8Array.fromBase64(response ?? ""));
        const { plain, xoauth2 } = this.#options;

        // accept matching PLAIN credentials
        if (mechanism === "PLAIN") {
            const isAccepted =
                plain !== undefined && decoded === `\0${plain.username}\0${plain.password}`;
            this.reply(
                line,
                isAccepted ? "235 2.7.0 authenticated" : "535 5.7.8 authentication failed",
            );
        }
        // accept a matching bearer token, else challenge before failing
        else if (mechanism === "XOAUTH2") {
            const isAccepted =
                xoauth2 !== undefined &&
                decoded === `user=${xoauth2.username}\x01auth=Bearer ${xoauth2.token}\x01\x01`;
            const reply = this.reply(
                line,
                isAccepted ? "235 2.7.0 authenticated" : `334 ${XOAUTH2_CHALLENGE}`,
            );
            this.#isChallenged = reply.startsWith("334");
        }
        // refuse other mechanisms
        else {
            this.reply(line, "504 5.5.4 mechanism not supported");
        }
    }

    /** Serve the session in TLS from now on, dropping the plain text sent after STARTTLS. */
    #upgrade(reader: ReadableStreamDefaultReader<Uint8Array>): void {
        // stop reading the plain text, dropping the lines already received behind STARTTLS
        reader.releaseLock();
        this.#received = "";

        // go on reading the socket as the TLS session the client upgraded it to
        this.#transcript.push("TLS");
        this.#isSecure = true;
        this.#reader = this.#socket.readable.getReader();
        void this.#listen(this.#reader);
    }
}
