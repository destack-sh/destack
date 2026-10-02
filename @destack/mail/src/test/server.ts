import { aligned } from "@destack/schema";
import net, { type Server, type Socket } from "node:net";
import tls from "node:tls";
import type { BearerCredentials, PlainCredentials, Security } from "../smtp/index.ts";
import type { TestCertificate } from "./certificate.ts";

/** The XOAUTH2 error challenge, base64 of the JSON status Gmail sends before its final 535. */
const XOAUTH2_CHALLENGE = btoa('{"status":"401","schemes":"bearer"}');

/** How a test server's sessions protect their bytes, with the certificate TLS presents. */
export type SmtpTestSecurity =
    | {
          /** Plain text throughout. */
          readonly security: Extract<Security, "none">;
      }
    | {
          /** TLS from the first byte or after STARTTLS. */
          readonly security: Exclude<Security, "none">;
          /** The certificate presented under TLS. */
          readonly certificate: TestCertificate;
      };

/** How a test server's sessions run. */
export type SmtpTestServerOptions = SmtpTestSecurity & SmtpTestScript;

/** The credentials a test server accepts and the replies it sends. */
export interface SmtpTestScript {
    /** The PLAIN credentials accepted. */
    readonly plain?: PlainCredentials;
    /** The XOAUTH2 credentials accepted. */
    readonly xoauth2?: BearerCredentials<string>;
    /** The scripted replies by client line: `greeting` for the banner, `.` for the data's end. */
    readonly answers?: Readonly<Record<string, string>>;
}

/** An in-process SMTP server on 127.0.0.1 that records every session and answers as scripted. */
export class SmtpTestServer implements AsyncDisposable {
    /** Each session's lines: `C:` from the client, `S:` from the server, `TLS` on a handshake. */
    readonly transcripts: string[][] = [];
    /** The content of each DATA command in order, without dot-stuffing and the ending dot. */
    readonly messages: string[] = [];
    /** How sessions run. */
    readonly #options: SmtpTestServerOptions;
    /** The server accepting sessions. */
    readonly #server: Server;
    /** The open sessions' sockets. */
    readonly #sockets = new Set<Socket>();

    /** Create a server that serves each connection. */
    private constructor(options: SmtpTestServerOptions) {
        // accept plain text or TLS from the first byte
        this.#options = options;
        this.#server =
            options.security === "tls"
                ? tls.createServer(
                      { key: options.certificate.key, cert: options.certificate.certificate },
                      (socket) => this.#accept(socket, true),
                  )
                : net.createServer((socket) => this.#accept(socket, false));
    }

    /** Listen on an ephemeral port of 127.0.0.1. */
    static async listen(options: SmtpTestServerOptions): Promise<SmtpTestServer> {
        const server = new SmtpTestServer(options);
        await new Promise<void>((resolve) => {
            server.#server.listen(0, "127.0.0.1", resolve);
        });

        return server;
    }

    /** The port the server listens on. */
    get port(): number {
        const address = this.#server.address();
        if (address === null || typeof address === "string") {
            throw new TypeError("test server listens on no port");
        }

        return address.port;
    }

    /** Close every session and stop listening. */
    async close(): Promise<void> {
        for (const socket of this.#sockets) {
            socket.destroy();
        }
        await new Promise((resolve) => {
            this.#server.close(resolve);
        });
    }

    /** Close the server. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Start a session on a new connection. */
    #accept(socket: Socket, isSecure: boolean): void {
        // track the socket and start the transcript
        this.#sockets.add(socket);
        socket.on("close", () => this.#sockets.delete(socket));
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
    readonly #options: SmtpTestServerOptions;
    /** The session's transcript. */
    readonly #transcript: string[];
    /** The server's received messages. */
    readonly #messages: string[];
    /** The socket, a TLS socket after STARTTLS. */
    #socket: Socket;
    /** Whether TLS protects the session. */
    #isSecure: boolean;
    /** The received text not yet split into lines. */
    #received = "";
    /** The data lines of the message being received, absent outside DATA. */
    #data: string[] | undefined;
    /** Whether the session waits on the empty line after an XOAUTH2 challenge. */
    #isChallenged = false;
    /** Handle received bytes. */
    readonly #receive = (chunk: Buffer): void => this.#read(chunk);

    /** Serve a socket. */
    constructor(
        options: SmtpTestServerOptions,
        transcript: string[],
        messages: string[],
        socket: Socket,
        isSecure: boolean,
    ) {
        // keep the session's state and read its socket
        this.#options = options;
        this.#transcript = transcript;
        this.#messages = messages;
        this.#socket = socket;
        this.#isSecure = isSecure;
        socket.on("data", this.#receive);
        socket.on("error", () => socket.destroy());
    }

    /** Write the scripted reply to a client line, or else the standard reply. */
    reply(line: string, standard: string): string {
        // write each line of the reply
        const reply = this.#options.answers?.[line] ?? standard;
        for (const replyLine of reply.split("\n")) {
            this.#transcript.push(`S: ${replyLine}`);
            this.#socket.write(`${replyLine}\r\n`);
        }

        return reply;
    }

    /** Split received bytes into lines and handle each. */
    #read(chunk: Buffer): void {
        this.#received += chunk.toString("utf8");
        let end = this.#received.indexOf("\r\n");
        while (end !== -1) {
            const line = this.#received.slice(0, end);
            this.#received = this.#received.slice(end + 2);
            this.#handle(line);
            end = this.#received.indexOf("\r\n");
        }
    }

    /** Handle one client line: a data line, the data's end or a command. */
    #handle(line: string): void {
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
            this.#command(line);
        }
    }

    /** Answer one command line, or the empty line after an XOAUTH2 challenge. */
    #command(line: string): void {
        // split the verb from its argument
        const verb = aligned(line.split(" "), 0).toUpperCase();
        const argument = line.slice(verb.length + 1);

        // fail the XOAUTH2 exchange after the client's empty answer
        if (this.#isChallenged) {
            this.#isChallenged = false;
            this.reply(line, "535 5.7.8 authentication failed");
        }
        // list the extensions, offering STARTTLS until TLS protects the session
        else if (verb === "EHLO") {
            const isOffered = this.#options.security === "starttls" && !this.#isSecure;
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
                this.#upgrade();
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
            this.#socket.end();
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

    /** Hand the plain socket to TLS, which serves the session from now on. */
    #upgrade(): void {
        // require the certificate STARTTLS presents
        const options = this.#options;
        if (options.security === "none") {
            throw new TypeError("plain test server cannot start tls");
        }
        const certificate = options.certificate;

        // stop reading the plain socket
        const plain = this.#socket;
        plain.off("data", this.#receive);

        // serve the TLS socket once its handshake completes
        const secure = new tls.TLSSocket(plain, {
            isServer: true,
            key: certificate.key,
            cert: certificate.certificate,
        });
        secure.on("secure", () => {
            this.#transcript.push("TLS");
            this.#isSecure = true;
        });
        secure.on("data", this.#receive);
        secure.on("error", () => secure.destroy());
        this.#socket = secure;
    }
}
