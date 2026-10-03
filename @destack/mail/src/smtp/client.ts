import net from "node:net";
import { Mailbox, type Envelope, type MimeMessage } from "../mime/index.ts";
import { SmtpConnection } from "./connection.ts";
import { SmtpError } from "./error.ts";
import { Extensions } from "./extensions.ts";
import type { Reply } from "./reply.ts";

/** How long a session waits on a silent server: a minute, far past a submission round trip. */
export const TIMEOUT_MILLISECONDS = 60_000;

/** How long one reply may take in all: two minutes, past a large message's scan at data end. */
export const DEADLINE_MILLISECONDS = 120_000;

/** An RFC 5321 address literal: an IPv4 address or a tagged IPv6 address in square brackets. */
const ADDRESS_LITERAL = /^\[(?:IPv6:)?([^\]]+)\]$/u;

/** How the session protects its bytes: implicit TLS, STARTTLS, or plain text on loopback only. */
export type Security = "tls" | "starttls" | "none";

/** The RFC 4616 PLAIN credentials: an authentication identity and its password. */
export interface PlainCredentials {
    /** The authentication identity. */
    readonly username: string;
    /** The password. */
    readonly password: string;
}

/** The XOAUTH2 credentials: a mailbox and its OAuth 2.0 access token. */
export interface BearerCredentials<Token = () => Promise<string>> {
    /** The mailbox the token authorises. */
    readonly username: string;
    /** The access token, or its supplier, which each session asks once. */
    readonly token: Token;
}

/** How the session authenticates: RFC 4616 PLAIN, XOAUTH2 with a bearer token, or not at all. */
export type Authentication =
    | ({ readonly kind: "plain" } & PlainCredentials)
    | ({ readonly kind: "xoauth2" } & BearerCredentials)
    | { readonly kind: "none" };

/** Where and how a client submits mail. */
export interface SmtpOptions {
    /** The server's host name or IP address. */
    readonly host: string;
    /** The server's port, such as 465 for implicit TLS or 587 for STARTTLS. */
    readonly port: number;
    /** How the session protects its bytes. */
    readonly security: Security;
    /** How the session authenticates. */
    readonly authentication: Authentication;
    /** The client's host name or address literal, which EHLO sends. */
    readonly helo: string;
    /** How long the session waits on a silent server, in milliseconds. */
    readonly timeout?: number;
    /** How long one reply, handshake or token may take in all, in milliseconds. */
    readonly deadline?: number;
    /** The PEM certificates trusted as authorities instead of the system's. */
    readonly certificateAuthorities?: readonly string[];
}

/** One recipient's reply to RCPT TO. */
export interface RecipientReply {
    /** The recipient's address. */
    readonly address: string;
    /** The server's reply. */
    readonly reply: Reply;
}

/** The server's replies to one message's recipients and to its data. */
export interface Submission {
    /** Each recipient's reply, in envelope order. */
    readonly recipients: readonly RecipientReply[];
    /** The reply ending the DATA phase, absent when the server accepted no recipient. */
    readonly data: Reply | undefined;
}

/** An RFC 5321 client that submits each message in a session of its own. */
export class SmtpClient {
    /** Where and how the client submits mail. */
    readonly #options: SmtpOptions;

    /** Create a client, refusing an invalid EHLO host and plain text to a host beyond loopback. */
    constructor(options: SmtpOptions) {
        // require a hostname or an address literal for EHLO
        if (!Mailbox.isDomain(options.helo) && !isAddressLiteral(options.helo)) {
            throw new SmtpError(
                "INVALID_OPTIONS",
                `helo ${JSON.stringify(options.helo)} is not a hostname or an address literal`,
            );
        }

        // require TLS off loopback
        if (options.security === "none" && !isLoopback(options.host)) {
            throw new SmtpError(
                "INSECURE",
                `security none requires a loopback host, not ${options.host}`,
            );
        }
        this.#options = options;
    }

    /** Submit one message: greet, secure, authenticate, then send the envelope and the content. */
    async submit(envelope: Envelope, message: MimeMessage): Promise<Submission> {
        // refuse an address that could break out of its command line
        for (const address of [envelope.sender, ...envelope.recipients]) {
            if (!Mailbox.isAddress(address)) {
                throw new SmtpError(
                    "INVALID_ENVELOPE",
                    `address ${JSON.stringify(address)} is not a dot-atom addr-spec`,
                );
            }
        }
        if (envelope.recipients.length === 0) {
            throw new SmtpError("INVALID_ENVELOPE", "envelope has no recipients");
        }

        // connect with the transport options alone
        const { host, port, security, timeout, deadline, certificateAuthorities } = this.#options;
        const connection = await SmtpConnection.open({
            host,
            port,
            security,
            timeout: timeout ?? TIMEOUT_MILLISECONDS,
            deadline: deadline ?? DEADLINE_MILLISECONDS,
            certificateAuthorities,
        });

        // run the session, closing the connection however it ends
        try {
            const extensions = await this.#greet(connection);
            await this.#authenticate(connection, extensions);

            return await this.#transfer(connection, envelope, message);
        } finally {
            connection.destroy();
        }
    }

    /** Read the greeting and send EHLO, upgrading to TLS and greeting again under STARTTLS. */
    async #greet(connection: SmtpConnection): Promise<Extensions> {
        // greet the server
        (await connection.read()).require("greeting");
        const greeting = `EHLO ${this.#options.helo}`;
        const extensions = Extensions.parse((await connection.command(greeting)).require("EHLO"));
        if (this.#options.security !== "starttls") {
            return extensions;
        }

        // upgrade, refusing a server that offers no STARTTLS
        if (!extensions.has("STARTTLS")) {
            throw new SmtpError("UNSUPPORTED", `${this.#endpoint()} offers no STARTTLS`);
        }
        (await connection.command("STARTTLS")).require("STARTTLS");
        await connection.upgrade();

        // greet again over TLS, forgetting the extensions listed in plain text
        return Extensions.parse((await connection.command(greeting)).require("EHLO"));
    }

    /** Authenticate with the configured mechanism, refusing a server that does not list it. */
    async #authenticate(connection: SmtpConnection, extensions: Extensions): Promise<void> {
        // skip a session without authentication
        const authentication = this.#options.authentication;
        if (authentication.kind === "none") {
            return;
        }

        // require the server to list the mechanism
        const mechanism = authentication.kind === "plain" ? "PLAIN" : "XOAUTH2";
        if (!extensions.has("AUTH", mechanism)) {
            throw new SmtpError("UNSUPPORTED", `${this.#endpoint()} offers no AUTH ${mechanism}`);
        }

        // authenticate with PLAIN's initial response
        if (authentication.kind === "plain") {
            const { username, password } = authentication;
            const response = encodeBase64(`\0${username}\0${password}`);
            (await connection.command(`AUTH PLAIN ${response}`)).require("AUTH PLAIN");
        }
        // authenticate with a bearer token, answering an error challenge with an empty line
        else {
            const token = await connection.race(authentication.token());
            const response = encodeBase64(
                `user=${authentication.username}\x01auth=Bearer ${token}\x01\x01`,
            );
            const reply = await connection.command(`AUTH XOAUTH2 ${response}`);
            const final = reply.code === 334 ? await connection.command("") : reply;
            final.require("AUTH XOAUTH2");
        }
    }

    /** Send the envelope and, once a recipient is accepted, the content, then quit. */
    async #transfer(
        connection: SmtpConnection,
        envelope: Envelope,
        message: MimeMessage,
    ): Promise<Submission> {
        // open the transaction and offer each recipient
        (await connection.command(`MAIL FROM:<${envelope.sender}>`)).require("MAIL FROM");
        const recipients: RecipientReply[] = [];
        for (const address of envelope.recipients) {
            recipients.push({ address, reply: await connection.command(`RCPT TO:<${address}>`) });
        }

        // send the content after 354, keep a refusal, and fail on any other reply
        let data: Reply | undefined;
        if (recipients.some((recipient) => recipient.reply.class === 2)) {
            const start = await connection.command("DATA");
            if (start.code === 354) {
                data = await this.#sendContent(connection, message);
            } else if (start.class === 4 || start.class === 5) {
                data = start;
            } else {
                throw new SmtpError(
                    "PROTOCOL",
                    `server answered DATA with ${start.code} instead of 354`,
                );
            }
        }

        // end the session, whose replies after the data change nothing
        await connection.close();

        return { recipients, data };
    }

    /** Send the content dot-stuffed and ended by a lone dot, and read the reply. */
    #sendContent(connection: SmtpConnection, message: MimeMessage): Promise<Reply> {
        // double each line's leading dot, as RFC 5321 section 4.5.2 requires
        const lines = message.content.split("\r\n");
        const stuffed = lines.map((line) => (line.startsWith(".") ? `.${line}` : line));
        connection.write(`${stuffed.join("\r\n")}.\r\n`);

        return connection.read();
    }

    /** Write the server's host and port, as failures report them. */
    #endpoint(): string {
        return `${this.#options.host}:${this.#options.port}`;
    }
}

/** Encode text as base64 of its UTF-8 bytes. */
function encodeBase64(text: string): string {
    return new TextEncoder().encode(text).toBase64();
}

/** Report whether text is an RFC 5321 address literal, such as `[192.0.2.1]` or `[IPv6:::1]`. */
function isAddressLiteral(text: string): boolean {
    const literal = ADDRESS_LITERAL.exec(text);
    const address = literal?.[1];

    return address !== undefined && net.isIP(address) === (text.startsWith("[IPv6:") ? 6 : 4);
}

/** Report whether a host is loopback: localhost, 127.0.0.0/8 or ::1. */
function isLoopback(host: string): boolean {
    return host === "localhost" || host === "::1" || (net.isIPv4(host) && host.startsWith("127."));
}
