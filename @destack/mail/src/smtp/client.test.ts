import net from "node:net";
import { beforeAll, expect, test } from "@destack/test";
import { MimeMessage, type Envelope } from "../mime/index.ts";
import { SmtpTestServer, TestCertificate } from "../test/index.ts";
import { SmtpClient, SmtpError, type Authentication, type SmtpOptions } from "./index.ts";

/** The PLAIN credentials the test servers accept. */
const PLAIN = { username: "ada", password: "secret" };
/** The XOAUTH2 credentials the test servers accept. */
const XOAUTH2 = { username: "ada@example.com", token: "ya29.token" };
/** The envelope most tests submit. */
const ENVELOPE: Envelope = { sender: "notices@destack.app", recipients: ["ada@example.com"] };
/** How often a trickling server sends one more byte, in milliseconds. */
const TRICKLE_MILLISECONDS = 5;

/** The greeting and first EHLO of a plain session. */
const GREETING = [
    "S: 220 mail.test ESMTP ready",
    "C: EHLO client.test",
    "S: 250-mail.test greets client.test",
    "S: 250-AUTH PLAIN XOAUTH2",
    "S: 250 ENHANCEDSTATUSCODES",
];
/** The PLAIN exchange of the accepted credentials. */
const AUTHENTICATED = ["C: AUTH PLAIN AGFkYQBzZWNyZXQ=", "S: 235 2.7.0 authenticated"];
/** The sender's acceptance. */
const SENDER = ["C: MAIL FROM:<notices@destack.app>", "S: 250 2.1.0 sender ok"];
/** The first recipient's acceptance. */
const RECIPIENT = ["C: RCPT TO:<ada@example.com>", "S: 250 2.1.5 recipient ok"];
/** The session's end. */
const QUIT = ["C: QUIT", "S: 221 2.0.0 bye"];
/** The data exchange and QUIT of an accepted message. */
const DATA = [
    "C: DATA",
    "S: 354 end data with <CR><LF>.<CR><LF>",
    "C: .",
    "S: 250 2.0.0 queued",
    ...QUIT,
];

/** The certificate the TLS servers present and the clients trust. */
let certificate: TestCertificate;
/** The message most tests submit. */
let message: MimeMessage;

beforeAll(async () => {
    certificate = await TestCertificate.generate();
    message = await MimeMessage.compose({
        from: "Destack <notices@destack.app>",
        to: ["ada@example.com"],
        subject: "New sign-in",
        date: new Date("2026-09-29T10:00:00Z"),
        key: "security-notice-01",
        text: "A new device signed in.",
    });
});

test("submit a message with PLAIN, keeping blind copies in the envelope only", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });
    const submission = await clientOf(server).submit(
        { sender: "notices@destack.app", recipients: ["ada@example.com", "audit@destack.app"] },
        message,
    );

    expect(submission).toEqual({
        recipients: [
            { address: "ada@example.com", reply: accepted("2.1.5", "recipient ok") },
            { address: "audit@destack.app", reply: accepted("2.1.5", "recipient ok") },
        ],
        data: accepted("2.0.0", "queued"),
    });
    expect(server.transcripts).toEqual([
        [
            ...GREETING,
            ...AUTHENTICATED,
            ...SENDER,
            ...RECIPIENT,
            "C: RCPT TO:<audit@destack.app>",
            "S: 250 2.1.5 recipient ok",
            ...DATA,
        ],
    ]);
    expect(server.messages).toEqual([message.content]);
});

test("submit with an XOAUTH2 token from the supplier", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", xoauth2: XOAUTH2 });
    const tokens: string[] = [];
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: async () => {
            tokens.push(XOAUTH2.token);

            return XOAUTH2.token;
        },
    };
    await clientOf(server, { authentication }).submit(ENVELOPE, message);

    // ask the supplier once per session
    expect(tokens).toEqual([XOAUTH2.token]);
    expect(server.transcripts).toEqual([
        [
            ...GREETING,
            "C: AUTH XOAUTH2 dXNlcj1hZGFAZXhhbXBsZS5jb20BYXV0aD1CZWFyZXIgeWEyOS50b2tlbgEB",
            "S: 235 2.7.0 authenticated",
            ...SENDER,
            ...RECIPIENT,
            ...DATA,
        ],
    ]);
    expect(server.messages).toEqual([message.content]);
});

test("reject a refused XOAUTH2 token after answering its challenge with an empty line", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", xoauth2: XOAUTH2 });
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: async () => "expired",
    };

    expect(await failureOf(clientOf(server, { authentication }).submit(ENVELOPE, message))).toEqual(
        {
            code: "REJECTED",
            message: "server answered AUTH XOAUTH2 with 535 authentication failed",
            reply: { code: 535, enhancedCode: "5.7.8", text: "authentication failed" },
        },
    );
    expect(server.transcripts).toEqual([
        [
            ...GREETING,
            "C: AUTH XOAUTH2 dXNlcj1hZGFAZXhhbXBsZS5jb20BYXV0aD1CZWFyZXIgZXhwaXJlZAEB",
            "S: 334 eyJzdGF0dXMiOiI0MDEiLCJzY2hlbWVzIjoiYmVhcmVyIn0=",
            "C: ",
            "S: 535 5.7.8 authentication failed",
        ],
    ]);
});

test("refuse a server that does not list the authentication mechanism", async () => {
    await using server = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { "EHLO client.test": "250-mail.test greets client.test\n250 AUTH XOAUTH2" },
    });

    expect(await failureOf(clientOf(server).submit(ENVELOPE, message))).toEqual({
        code: "UNSUPPORTED",
        message: `127.0.0.1:${server.port} offers no AUTH PLAIN`,
        reply: undefined,
    });
    expect(server.transcripts).toEqual([
        [
            "S: 220 mail.test ESMTP ready",
            "C: EHLO client.test",
            "S: 250-mail.test greets client.test",
            "S: 250 AUTH XOAUTH2",
        ],
    ]);
});

test("return each recipient's reply verbatim, sending data once one is accepted", async () => {
    await using server = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: {
            "RCPT TO:<bob@example.com>": "451 4.3.0 mailbox busy",
            "RCPT TO:<eve@example.com>":
                "550-5.1.1 mailbox unavailable\n550 5.1.1 see https://mail.test/550",
        },
    });
    const submission = await clientOf(server).submit(
        {
            sender: "notices@destack.app",
            recipients: ["bob@example.com", "ada@example.com", "eve@example.com"],
        },
        message,
    );

    expect(submission).toEqual({
        recipients: [
            {
                address: "bob@example.com",
                reply: { code: 451, enhancedCode: "4.3.0", text: "mailbox busy" },
            },
            { address: "ada@example.com", reply: accepted("2.1.5", "recipient ok") },
            {
                address: "eve@example.com",
                reply: {
                    code: 550,
                    enhancedCode: "5.1.1",
                    text: "mailbox unavailable\nsee https://mail.test/550",
                },
            },
        ],
        data: accepted("2.0.0", "queued"),
    });
    expect(server.messages).toEqual([message.content]);
});

test("keep the text of replies without an enhanced code of their class", async () => {
    await using server = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: {
            "RCPT TO:<ada@example.com>": "250 recipient ok",
            "RCPT TO:<bob@example.com>": "250 5.1.1 forwarded",
        },
    });
    const submission = await clientOf(server).submit(
        { sender: "notices@destack.app", recipients: ["ada@example.com", "bob@example.com"] },
        message,
    );

    expect(submission).toEqual({
        recipients: [
            {
                address: "ada@example.com",
                reply: { code: 250, enhancedCode: undefined, text: "recipient ok" },
            },
            {
                address: "bob@example.com",
                reply: { code: 250, enhancedCode: undefined, text: "5.1.1 forwarded" },
            },
        ],
        data: accepted("2.0.0", "queued"),
    });
});

test("skip data and quit when the server accepts no recipient", async () => {
    await using server = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { "RCPT TO:<ada@example.com>": "550 5.1.1 no such user" },
    });
    const submission = await clientOf(server).submit(ENVELOPE, message);

    expect(submission).toEqual({
        recipients: [
            {
                address: "ada@example.com",
                reply: { code: 550, enhancedCode: "5.1.1", text: "no such user" },
            },
        ],
        data: undefined,
    });
    expect(server.transcripts).toEqual([
        [
            ...GREETING,
            ...AUTHENTICATED,
            ...SENDER,
            "C: RCPT TO:<ada@example.com>",
            "S: 550 5.1.1 no such user",
            ...QUIT,
        ],
    ]);
    expect(server.messages).toEqual([]);
});

test("return the reply refusing DATA or the data's end verbatim", async () => {
    await using refusing = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { DATA: "451 4.3.1 insufficient system storage" },
    });
    await using ending = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { ".": "550 5.7.1 message content rejected" },
    });
    const refused = await clientOf(refusing).submit(ENVELOPE, message);
    const ended = await clientOf(ending).submit(ENVELOPE, message);

    expect([refused.data, ended.data]).toEqual([
        { code: 451, enhancedCode: "4.3.1", text: "insufficient system storage" },
        { code: 550, enhancedCode: "5.7.1", text: "message content rejected" },
    ]);
    expect([...refusing.transcripts, ...ending.transcripts]).toEqual([
        [
            ...GREETING,
            ...AUTHENTICATED,
            ...SENDER,
            ...RECIPIENT,
            "C: DATA",
            "S: 451 4.3.1 insufficient system storage",
            ...QUIT,
        ],
        [
            ...GREETING,
            ...AUTHENTICATED,
            ...SENDER,
            ...RECIPIENT,
            "C: DATA",
            "S: 354 end data with <CR><LF>.<CR><LF>",
            "C: .",
            "S: 550 5.7.1 message content rejected",
            ...QUIT,
        ],
    ]);
    expect([refusing.messages, ending.messages]).toEqual([[], [message.content]]);
});

test("fail on a positive reply to DATA", async () => {
    await using server = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { DATA: "250 2.0.0 ok" },
    });

    expect(await failureOf(clientOf(server).submit(ENVELOPE, message))).toEqual({
        code: "PROTOCOL",
        message: "server answered DATA with 250 instead of 354",
        reply: undefined,
    });
    expect(server.transcripts).toEqual([
        [...GREETING, ...AUTHENTICATED, ...SENDER, ...RECIPIENT, "C: DATA", "S: 250 2.0.0 ok"],
    ]);
    expect(server.messages).toEqual([]);
});

test("dot-stuff content lines that start with a dot", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });
    const dotted = await MimeMessage.compose({
        from: "notices@destack.app",
        to: ["ada@example.com"],
        subject: "Dots",
        date: new Date("2026-09-29T10:00:00Z"),
        key: "dots",
        text: ".first\n..second\n.\nlast",
    });
    await clientOf(server).submit(ENVELOPE, dotted);

    // receive the content unchanged, a lone dot line included
    expect(server.messages).toEqual([dotted.content]);
});

test("upgrade with STARTTLS and greet again before authenticating", async () => {
    await using server = await SmtpTestServer.listen({
        security: "starttls",
        certificate,
        plain: PLAIN,
    });
    await clientOf(server, { security: "starttls" }).submit(ENVELOPE, message);

    expect(server.transcripts).toEqual([
        [
            "S: 220 mail.test ESMTP ready",
            "C: EHLO client.test",
            "S: 250-mail.test greets client.test",
            "S: 250-AUTH PLAIN XOAUTH2",
            "S: 250-STARTTLS",
            "S: 250 ENHANCEDSTATUSCODES",
            "C: STARTTLS",
            "S: 220 2.0.0 ready to start TLS",
            "TLS",
            ...GREETING.slice(1),
            ...AUTHENTICATED,
            ...SENDER,
            ...RECIPIENT,
            ...DATA,
        ],
    ]);
    expect(server.messages).toEqual([message.content]);
});

test("submit over implicit TLS", async () => {
    await using server = await SmtpTestServer.listen({
        security: "tls",
        certificate,
        plain: PLAIN,
    });
    await clientOf(server, { security: "tls" }).submit(ENVELOPE, message);

    expect(server.transcripts).toEqual([
        ["TLS", ...GREETING, ...AUTHENTICATED, ...SENDER, ...RECIPIENT, ...DATA],
    ]);
    expect(server.messages).toEqual([message.content]);
});

test("refuse a server that offers no STARTTLS", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });
    const failure = await failureOf(
        clientOf(server, { security: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        code: "UNSUPPORTED",
        message: `127.0.0.1:${server.port} offers no STARTTLS`,
        reply: undefined,
    });
    expect(server.transcripts).toEqual([GREETING]);
});

test("reject a refused STARTTLS with the server's reply", async () => {
    await using server = await SmtpTestServer.listen({
        security: "starttls",
        certificate,
        plain: PLAIN,
        answers: { STARTTLS: "454 4.7.0 TLS not available" },
    });
    const failure = await failureOf(
        clientOf(server, { security: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        code: "REJECTED",
        message: "server answered STARTTLS with 454 TLS not available",
        reply: { code: 454, enhancedCode: "4.7.0", text: "TLS not available" },
    });
    expect(server.transcripts).toEqual([
        [
            "S: 220 mail.test ESMTP ready",
            "C: EHLO client.test",
            "S: 250-mail.test greets client.test",
            "S: 250-AUTH PLAIN XOAUTH2",
            "S: 250-STARTTLS",
            "S: 250 ENHANCEDSTATUSCODES",
            "C: STARTTLS",
            "S: 454 4.7.0 TLS not available",
        ],
    ]);
});

test("refuse replies injected ahead of the TLS handshake", async () => {
    await using server = await SmtpTestServer.listen({
        security: "starttls",
        certificate,
        plain: PLAIN,
        answers: { STARTTLS: "220 2.0.0 ready to start TLS\n250 2.0.0 injected" },
    });
    const failure = await failureOf(
        clientOf(server, { security: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        code: "PROTOCOL",
        message: "server sent data after its STARTTLS reply",
        reply: undefined,
    });
});

test("refuse a certificate no trusted authority signed", async () => {
    await using server = await SmtpTestServer.listen({
        security: "tls",
        certificate,
        plain: PLAIN,
    });
    const other = await TestCertificate.generate();
    const client = clientOf(server, {
        security: "tls",
        certificateAuthorities: [other.certificate],
    });

    expect(await failureOf(client.submit(ENVELOPE, message))).toEqual({
        code: "CONNECTION",
        message: `connection to 127.0.0.1:${server.port} failed: self signed certificate`,
        reply: undefined,
    });
});

test("refuse plain text to a host beyond loopback", () => {
    expect(() => clientOf({ port: 25 }, { host: "mail.example.com" })).toThrow(
        new SmtpError("INSECURE", "security none requires a loopback host, not mail.example.com"),
    );
});

test("refuse an EHLO host that is no hostname or address literal", () => {
    // accept hostnames and address literals, and refuse anything that could inject a command
    const helos = [
        "client.test",
        "[127.0.0.1]",
        "[IPv6:::1]",
        "client.test\r\nRSET",
        "client test",
        "[IPv6:127.0.0.1]",
        "",
    ];
    const outcomes = helos.map((helo) => {
        try {
            clientOf({ port: 25 }, { helo });

            return "accepted";
        } catch (error) {
            return error instanceof SmtpError
                ? { code: error.code, message: error.message }
                : error;
        }
    });

    expect(outcomes).toEqual([
        "accepted",
        "accepted",
        "accepted",
        {
            code: "INVALID_OPTIONS",
            message: 'helo "client.test\\r\\nRSET" is not a hostname or an address literal',
        },
        {
            code: "INVALID_OPTIONS",
            message: 'helo "client test" is not a hostname or an address literal',
        },
        {
            code: "INVALID_OPTIONS",
            message: 'helo "[IPv6:127.0.0.1]" is not a hostname or an address literal',
        },
        { code: "INVALID_OPTIONS", message: 'helo "" is not a hostname or an address literal' },
    ]);
});

test("reject a refused greeting, credentials or sender with the server's reply", async () => {
    await using greeting = await SmtpTestServer.listen({
        security: "none",
        answers: { greeting: "554 5.3.2 service unavailable" },
    });
    await using credentials = await SmtpTestServer.listen({ security: "none" });
    await using sender = await SmtpTestServer.listen({
        security: "none",
        plain: PLAIN,
        answers: { "MAIL FROM:<notices@destack.app>": "553 5.7.1 sender not allowed" },
    });
    const failures = await Promise.all(
        [greeting, credentials, sender].map((server) =>
            failureOf(clientOf(server).submit(ENVELOPE, message)),
        ),
    );

    expect(failures).toEqual([
        {
            code: "REJECTED",
            message: "server answered greeting with 554 service unavailable",
            reply: { code: 554, enhancedCode: "5.3.2", text: "service unavailable" },
        },
        {
            code: "REJECTED",
            message: "server answered AUTH PLAIN with 535 authentication failed",
            reply: { code: 535, enhancedCode: "5.7.8", text: "authentication failed" },
        },
        {
            code: "REJECTED",
            message: "server answered MAIL FROM with 553 sender not allowed",
            reply: { code: 553, enhancedCode: "5.7.1", text: "sender not allowed" },
        },
    ]);
});

test("fail on malformed, mismatched and overlong replies", async () => {
    await using malformed = await SmtpTestServer.listen({
        security: "none",
        answers: { greeting: "hello" },
    });
    await using mismatched = await SmtpTestServer.listen({
        security: "none",
        answers: { greeting: "220-mail.test ESMTP\n250 ready" },
    });
    await using overlong = await SmtpTestServer.listen({
        security: "none",
        answers: { greeting: `220 ${"x".repeat(600)}` },
    });
    await using unended = await listenRaw((socket) => socket.write(`220 ${"x".repeat(600)}`));
    const failures = [
        await failureOf(clientOf(malformed).submit(ENVELOPE, message)),
        await failureOf(clientOf(mismatched).submit(ENVELOPE, message)),
        await failureOf(clientOf(overlong).submit(ENVELOPE, message)),
        await failureOf(clientOf(unended).submit(ENVELOPE, message)),
    ];

    expect(failures).toEqual([
        {
            code: "PROTOCOL",
            message: 'server sent "hello", which is no reply line',
            reply: undefined,
        },
        {
            code: "PROTOCOL",
            message: "server continued a 220 reply with a 250 line",
            reply: undefined,
        },
        { code: "PROTOCOL", message: "server sent a reply line over 512 octets", reply: undefined },
        { code: "PROTOCOL", message: "server sent a reply line over 512 octets", reply: undefined },
    ]);
});

test("fail on a silent server, a trickling server, a stalled token and a refused connection", async () => {
    await using silent = await listenRaw(() => undefined);
    await using trickling = await listenRaw((socket) => {
        // send one more byte of an endless greeting every few milliseconds
        const timer = setInterval(() => socket.write("x"), TRICKLE_MILLISECONDS);
        socket.on("close", () => clearInterval(timer));
        socket.write("220 ");
    });
    await using stalled = await SmtpTestServer.listen({ security: "none", xoauth2: XOAUTH2 });
    const closed = await listenRaw(() => undefined);
    await closed.close();
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: () => new Promise(() => {}),
    };
    const failures = [
        await failureOf(clientOf(silent, { timeout: 20 }).submit(ENVELOPE, message)),
        await failureOf(
            clientOf(trickling, { timeout: 1000, deadline: 50 }).submit(ENVELOPE, message),
        ),
        await failureOf(
            clientOf(stalled, { authentication, deadline: 50 }).submit(ENVELOPE, message),
        ),
        await failureOf(clientOf(closed).submit(ENVELOPE, message)),
    ];

    expect(failures).toEqual([
        {
            code: "TIMEOUT",
            message: `127.0.0.1:${silent.port} sent nothing for 20 ms`,
            reply: undefined,
        },
        {
            code: "TIMEOUT",
            message: `127.0.0.1:${trickling.port} missed the 50 ms deadline`,
            reply: undefined,
        },
        {
            code: "TIMEOUT",
            message: `127.0.0.1:${stalled.port} missed the 50 ms deadline`,
            reply: undefined,
        },
        {
            code: "CONNECTION",
            message: `connection to 127.0.0.1:${closed.port} failed: connect ECONNREFUSED 127.0.0.1:${closed.port}`,
            reply: undefined,
        },
    ]);
});

test("refuse envelope addresses that could inject commands, and empty envelopes", async () => {
    const client = clientOf({ port: 25 });
    const failures = await Promise.all([
        failureOf(client.submit({ ...ENVELOPE, sender: "a@b.example>\r\nRSET" }, message)),
        failureOf(
            client.submit(
                { ...ENVELOPE, recipients: ["ada@example.com", "<eve@example.com>"] },
                message,
            ),
        ),
        failureOf(client.submit({ ...ENVELOPE, recipients: [] }, message)),
    ]);

    expect(failures).toEqual([
        {
            code: "INVALID_ENVELOPE",
            message: 'address "a@b.example>\\r\\nRSET" is not a dot-atom addr-spec',
            reply: undefined,
        },
        {
            code: "INVALID_ENVELOPE",
            message: 'address "<eve@example.com>" is not a dot-atom addr-spec',
            reply: undefined,
        },
        { code: "INVALID_ENVELOPE", message: "envelope has no recipients", reply: undefined },
    ]);
});

/** Create a client for a server on 127.0.0.1, with PLAIN in plain text by default. */
function clientOf(
    server: { readonly port: number },
    options: Partial<SmtpOptions> = {},
): SmtpClient {
    return new SmtpClient({
        host: "127.0.0.1",
        port: server.port,
        security: "none",
        authentication: { kind: "plain", ...PLAIN },
        helo: "client.test",
        certificateAuthorities: [certificate.certificate],
        ...options,
    });
}

/** Build a positive completion reply. */
function accepted(enhancedCode: string, text: string) {
    return { code: 250, enhancedCode, text };
}

/** Listen on 127.0.0.1, serving each connection's raw socket. */
async function listenRaw(
    serve: (socket: net.Socket) => void,
): Promise<{ port: number; close: () => Promise<void> } & AsyncDisposable> {
    // accept connections, keeping each socket until the server closes
    const sockets = new Set<net.Socket>();
    const server = net.createServer((socket) => {
        sockets.add(socket);
        socket.on("error", () => socket.destroy());
        serve(socket);
    });
    await new Promise<void>((resolve) => {
        server.listen(0, "127.0.0.1", resolve);
    });

    // read the listening port
    const address = server.address();
    if (address === null || typeof address === "string") {
        throw new TypeError("raw server listens on no port");
    }

    // close every socket, then the server
    const close = async () => {
        for (const socket of sockets) {
            socket.destroy();
        }
        await new Promise((resolve) => {
            server.close(resolve);
        });
    };

    return { port: address.port, close, [Symbol.asyncDispose]: close };
}

/** Read the code, message and reply a submission fails with. */
async function failureOf(submitted: Promise<unknown>): Promise<Record<string, unknown>> {
    try {
        await submitted;
    } catch (error) {
        if (error instanceof SmtpError) {
            return { code: error.code, message: error.message, reply: error.reply };
        }
        throw error;
    }
    throw new Error("submission succeeded");
}
