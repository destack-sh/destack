import type { Environment, SecureTransport } from "@destack/environment";
import { Mailbox, type Envelope, type MimeMessage } from "../mime/index.ts";
import { SmtpConnection } from "./connection.ts";
import { MessageError } from "../error/error.ts";
import { Extensions } from "./extension.ts";
import type { Reply } from "./reply.ts";

/** How long each step of a session may take, in milliseconds (RFC 5321 4.5.3.2). */
export const STEP_TIMEOUTS = Object.freeze({
    /** The initial 220 greeting, five minutes (4.5.3.2.1). */
    greeting: 5 * 60_000,
    /** The reply to MAIL, five minutes (4.5.3.2.2). */
    mail: 5 * 60_000,
    /** The reply to RCPT, five minutes (4.5.3.2.3). */
    rcpt: 5 * 60_000,
    /** The reply to DATA, two minutes (4.5.3.2.4). */
    data: 2 * 60_000,
    /** Sending one block of the content, three minutes (4.5.3.2.5). */
    block: 3 * 60_000,
    /** The reply to the content's final dot, ten minutes (4.5.3.2.6). */
    termination: 10 * 60_000,
    /** Every other step, such as EHLO, STARTTLS, AUTH, QUIT, the dial and a token, five minutes. */
    command: 5 * 60_000,
});

/** The bytes of one content block, 64 KiB: half a second at 1 Mbit/s, inside a block's timeout. */
const BLOCK_LENGTH = 64 * 1024;

/** An IPv4 address in dotted-decimal form, each part 0 to 255. */
const IPV4 = /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/u;

/** An RFC 5321 address literal: an IPv4 address or a tagged IPv6 address in square brackets. */
const ADDRESS_LITERAL = /^\[(?:IPv6:)?([^\]]+)\]$/u;

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
    /** How the session protects its bytes: TLS at once, STARTTLS, or plain text on loopback. */
    readonly secureTransport: SecureTransport;
    /** How the session authenticates. */
    readonly authentication: Authentication;
    /** The client's host name or address literal, which EHLO sends. */
    readonly helo: string;
    /** The network the client dials and the clock its timeouts run on. */
    readonly environment: Environment;
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
    readonly options: SmtpOptions;

    /** Create a client, refusing an invalid EHLO host and plain text to a host beyond loopback. */
    constructor(options: SmtpOptions) {
        // require a hostname or an address literal for EHLO
        if (!Mailbox.isDomain(options.helo) && !isAddressLiteral(options.helo)) {
            throw new TypeError(
                `helo ${JSON.stringify(options.helo)} is not a hostname or an address literal`,
            );
        }

        // require TLS off loopback
        if (options.secureTransport === "off" && !isLoopback(options.host)) {
            throw new TypeError(`plain text requires a loopback host, not ${options.host}`);
        }
        this.options = options;
    }

    /** Submit one message: greet, secure, authenticate, then send the envelope and the content. */
    async submit(envelope: Envelope, message: MimeMessage): Promise<Submission> {
        // refuse an address that could break out of its command line
        for (const address of [envelope.sender, ...envelope.recipients]) {
            if (!Mailbox.isAddress(address)) {
                throw MessageError.of(
                    "ENVELOPE_INVALID",
                    `address ${JSON.stringify(address)} is not a dot-atom addr-spec`,
                );
            }
        }
        if (envelope.recipients.length === 0) {
            throw MessageError.of("ENVELOPE_INVALID", "envelope has no recipients");
        }

        // connect with only the transport options
        const { host, port, secureTransport, environment } = this.options;
        const connection = await SmtpConnection.open({
            host,
            port,
            secureTransport,
            timeout: STEP_TIMEOUTS.command,
            environment,
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
        (await connection.read(STEP_TIMEOUTS.greeting)).require("greeting");
        const greeting = `EHLO ${this.options.helo}`;
        const extensions = Extensions.parse(
            (await connection.command(greeting, STEP_TIMEOUTS.command)).require("EHLO"),
        );
        if (this.options.secureTransport !== "starttls") {
            return extensions;
        }

        // upgrade, refusing a server that offers no STARTTLS
        if (!extensions.has("STARTTLS")) {
            throw MessageError.of(
                "EXTENSION_UNSUPPORTED",
                `${this.#address()} does not offer STARTTLS`,
            );
        }
        (await connection.command("STARTTLS", STEP_TIMEOUTS.command)).require("STARTTLS");
        await connection.upgrade();

        // greet again over TLS, forgetting the extensions listed in plain text
        return Extensions.parse(
            (await connection.command(greeting, STEP_TIMEOUTS.command)).require("EHLO"),
        );
    }

    /** Authenticate with the configured mechanism, refusing a server that does not list it. */
    async #authenticate(connection: SmtpConnection, extensions: Extensions): Promise<void> {
        // skip a session without authentication
        const authentication = this.options.authentication;
        if (authentication.kind === "none") {
            return;
        }

        // require the server to list the mechanism
        const mechanism = authentication.kind === "plain" ? "PLAIN" : "XOAUTH2";
        if (!extensions.has("AUTH", mechanism)) {
            throw MessageError.of(
                "EXTENSION_UNSUPPORTED",
                `${this.#address()} does not offer AUTH ${mechanism}`,
            );
        }

        // authenticate with PLAIN's initial response
        if (authentication.kind === "plain") {
            const { username, password } = authentication;
            const response = encodeBase64(`\0${username}\0${password}`);
            const reply = await connection.command(`AUTH PLAIN ${response}`, STEP_TIMEOUTS.command);
            reply.require("AUTH PLAIN");
        }
        // authenticate with a bearer token, answering an error challenge with an empty line
        else {
            const token = await connection.race(authentication.token(), STEP_TIMEOUTS.command);
            const response = encodeBase64(
                `user=${authentication.username}\x01auth=Bearer ${token}\x01\x01`,
            );
            const reply = await connection.command(
                `AUTH XOAUTH2 ${response}`,
                STEP_TIMEOUTS.command,
            );
            const final =
                reply.code === 334 ? await connection.command("", STEP_TIMEOUTS.command) : reply;
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
        const mail = await connection.command(`MAIL FROM:<${envelope.sender}>`, STEP_TIMEOUTS.mail);
        mail.require("MAIL FROM");
        const recipients: RecipientReply[] = [];
        for (const address of envelope.recipients) {
            const reply = await connection.command(`RCPT TO:<${address}>`, STEP_TIMEOUTS.rcpt);
            recipients.push({ address, reply });
        }

        // send the content after 354, record a refusal, and fail on any other reply
        let data: Reply | undefined;
        if (recipients.some((recipient) => recipient.reply.class === 2)) {
            const start = await connection.command("DATA", STEP_TIMEOUTS.data);
            if (start.code === 354) {
                data = await this.#sendContent(connection, message);
            } else if (start.class === 4 || start.class === 5) {
                data = start;
            } else {
                throw MessageError.of(
                    "RESPONSE_INVALID",
                    `server responded to DATA with ${start.code} instead of 354`,
                );
            }
        }

        // end the session, whose replies after the data change nothing
        await connection.close(STEP_TIMEOUTS.command);

        return { recipients, data };
    }

    /** Send the content dot-stuffed and ended by a lone dot, and read the reply. */
    async #sendContent(connection: SmtpConnection, message: MimeMessage): Promise<Reply> {
        // double each line's leading dot, as RFC 5321 section 4.5.2 requires
        const lines = message.content.split("\r\n");
        const stuffed = lines.map((line) => (line.startsWith(".") ? `.${line}` : line));
        const content = new TextEncoder().encode(`${stuffed.join("\r\n")}.\r\n`);

        // send each block within its timeout
        for (let start = 0; start < content.length; start += BLOCK_LENGTH) {
            const block = content.subarray(start, start + BLOCK_LENGTH);
            await connection.race(connection.write(block), STEP_TIMEOUTS.block);
        }

        return connection.read(STEP_TIMEOUTS.termination);
    }

    /** Write the server's host and port, as failures report them. */
    #address(): string {
        return `${this.options.host}:${this.options.port}`;
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

    return (
        address !== undefined && (text.startsWith("[IPv6:") ? isIPv6(address) : IPV4.test(address))
    );
}

/** Report whether text is an IPv6 address, as a URL's host parses it. */
function isIPv6(text: string): boolean {
    return text.includes(":") && URL.canParse(`http://[${text}]/`);
}

/** Report whether a host is loopback: localhost, 127.0.0.0/8 or ::1. */
function isLoopback(host: string): boolean {
    return host === "localhost" || host === "::1" || (IPV4.test(host) && host.startsWith("127."));
}
