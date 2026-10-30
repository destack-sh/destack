import { Simulation, type SimulatedClock } from "@destack/environment/simulation";
import type { Environment, Server, Socket } from "@destack/environment";
import { expect, test } from "@destack/test";
import { type Draft, MimeMessage, type Envelope } from "../mime/index.ts";
import { TestMailServer } from "../test/index.ts";
import { MessageError } from "../error/index.ts";
import { SmtpClient, type Authentication, type SmtpOptions, type Submission } from "./index.ts";

/** The PLAIN credentials the test servers accept. */
const PLAIN = { username: "ada", password: "secret" };
/** The XOAUTH2 credentials the test servers accept. */
const XOAUTH2 = { username: "ada@example.com", token: "ya29.token" };
/** The envelope most tests submit. */
const ENVELOPE: Envelope = { sender: "notices@destack.app", recipients: ["ada@example.com"] };
/** How often a trickling server sends one more byte, below the line limit within five minutes. */
const TRICKLE_MILLISECONDS = 1000;
/** The time every simulation starts at. */
const START = Date.UTC(2026, 8, 29, 10);

/** The draft of the message most tests submit. */
const DRAFT: Draft = {
    from: "Destack <notices@destack.app>",
    to: ["ada@example.com"],
    subject: "New sign-in",
    date: new Date("2026-09-29T10:00:00Z"),
    key: "security-notice-01",
    text: "A new device signed in.",
};

/** A simulated world: the mail server's node, the client's environment, and the clock of both. */
interface World {
    /** The simulation. */
    readonly simulation: Simulation;
    /** The clock both run on. */
    readonly clock: SimulatedClock;
    /** The server the mail server listens with. */
    readonly server: Server;
    /** The client's environment. */
    readonly environment: Environment;
}

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

test("submit a message with PLAIN, with blind copies in the envelope only", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const submission = await createClient(world, server).submit(
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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        xoauth2: XOAUTH2,
    });
    const tokens: string[] = [];
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: async () => {
            tokens.push(XOAUTH2.token);

            return XOAUTH2.token;
        },
    };
    await createClient(world, server, { authentication }).submit(ENVELOPE, message);

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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        xoauth2: XOAUTH2,
    });
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: async () => "expired",
    };

    expect(
        await readFailure(
            createClient(world, server, { authentication }).submit(ENVELOPE, message),
        ),
    ).toEqual({
        refusal: [
            "UNPROCESSABLE_CONTENT",
            "COMMAND_REJECTED",
            "server responded to AUTH XOAUTH2 with 535 authentication failed",
        ],
        reply: { code: 535, enhancedCode: "5.7.8", text: "authentication failed" },
    });
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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { "EHLO client.test": "250-mail.test greets client.test\n250 AUTH XOAUTH2" },
    });

    expect(await readFailure(createClient(world, server).submit(ENVELOPE, message))).toEqual({
        refusal: [
            "BAD_GATEWAY",
            "EXTENSION_UNSUPPORTED",
            `127.0.0.1:${server.port} does not offer AUTH PLAIN`,
        ],
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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: {
            "RCPT TO:<bob@example.com>": "451 4.3.0 mailbox busy",
            "RCPT TO:<eve@example.com>":
                "550-5.1.1 mailbox unavailable\n550 5.1.1 see https://mail.test/550",
        },
    });
    const submission = await createClient(world, server).submit(
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

test("read the text of replies without an enhanced code of their class", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: {
            "RCPT TO:<ada@example.com>": "250 recipient ok",
            "RCPT TO:<bob@example.com>": "250 5.1.1 forwarded",
        },
    });
    const submission = await createClient(world, server).submit(
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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { "RCPT TO:<ada@example.com>": "550 5.1.1 no such user" },
    });
    const submission = await createClient(world, server).submit(ENVELOPE, message);

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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using refusing = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { DATA: "451 4.3.1 insufficient system storage" },
    });
    await using ending = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { ".": "550 5.7.1 message content rejected" },
    });
    const refused = await createClient(world, refusing).submit(ENVELOPE, message);
    const ended = await createClient(world, ending).submit(ENVELOPE, message);

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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { DATA: "250 2.0.0 ok" },
    });

    expect(await readFailure(createClient(world, server).submit(ENVELOPE, message))).toEqual({
        refusal: [
            "BAD_GATEWAY",
            "RESPONSE_INVALID",
            "server responded to DATA with 250 instead of 354",
        ],
        reply: undefined,
    });
    expect(server.transcripts).toEqual([
        [...GREETING, ...AUTHENTICATED, ...SENDER, ...RECIPIENT, "C: DATA", "S: 250 2.0.0 ok"],
    ]);
    expect(server.messages).toEqual([]);
});

test("dot-stuff content lines that start with a dot", async () => {
    const world = simulate();
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const dotted = await MimeMessage.compose(
        {
            from: "notices@destack.app",
            to: ["ada@example.com"],
            subject: "Dots",
            date: new Date("2026-09-29T10:00:00Z"),
            key: "dots",
            text: ".first\n..second\n.\nlast",
        },
        world.environment.crypto,
    );
    await createClient(world, server).submit(ENVELOPE, dotted);

    // receive the content unchanged, a lone dot line included
    expect(server.messages).toEqual([dotted.content]);
});

test("upgrade with STARTTLS and greet again before authenticating", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "starttls",
        plain: PLAIN,
    });
    await createClient(world, server, { secureTransport: "starttls" }).submit(ENVELOPE, message);

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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "on",
        plain: PLAIN,
    });
    await createClient(world, server, { secureTransport: "on" }).submit(ENVELOPE, message);

    expect(server.transcripts).toEqual([
        ["TLS", ...GREETING, ...AUTHENTICATED, ...SENDER, ...RECIPIENT, ...DATA],
    ]);
    expect(server.messages).toEqual([message.content]);
});

test("refuse a server that does not support STARTTLS", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const failure = await readFailure(
        createClient(world, server, { secureTransport: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        refusal: [
            "BAD_GATEWAY",
            "EXTENSION_UNSUPPORTED",
            `127.0.0.1:${server.port} does not offer STARTTLS`,
        ],
        reply: undefined,
    });
    expect(server.transcripts).toEqual([GREETING]);
});

test("reject a refused STARTTLS with the server's reply", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "starttls",
        plain: PLAIN,
        answers: { STARTTLS: "454 4.7.0 TLS not available" },
    });
    const failure = await readFailure(
        createClient(world, server, { secureTransport: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        refusal: [
            "UNPROCESSABLE_CONTENT",
            "COMMAND_REJECTED",
            "server responded to STARTTLS with 454 TLS not available",
        ],
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
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using server = await TestMailServer.listen(world.server, {
        secureTransport: "starttls",
        plain: PLAIN,
        answers: { STARTTLS: "220 2.0.0 ready to start TLS\n250 2.0.0 injected" },
    });
    const failure = await readFailure(
        createClient(world, server, { secureTransport: "starttls" }).submit(ENVELOPE, message),
    );

    expect(failure).toEqual({
        refusal: ["BAD_GATEWAY", "RESPONSE_INVALID", "server sent data after its STARTTLS reply"],
        reply: undefined,
    });
});

test("refuse plain text to a host beyond loopback", async () => {
    const world = simulate();
    expect(() => createClient(world, { port: 25 }, { host: "mail.example.com" })).toThrow(
        new TypeError("plain text requires a loopback host, not mail.example.com"),
    );
});

test("refuse an EHLO host that is not a hostname or address literal", async () => {
    const world = simulate();
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
            createClient(world, { port: 25 }, { helo });

            return "accepted";
        } catch (error) {
            return error instanceof TypeError ? error.message : error;
        }
    });

    expect(outcomes).toEqual([
        "accepted",
        "accepted",
        "accepted",
        'helo "client.test\\r\\nRSET" is not a hostname or an address literal',
        'helo "client test" is not a hostname or an address literal',
        'helo "[IPv6:127.0.0.1]" is not a hostname or an address literal',
        'helo "" is not a hostname or an address literal',
    ]);
});

test("reject a refused greeting, credentials or sender with the server's reply", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using greeting = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        answers: { greeting: "554 5.3.2 service unavailable" },
    });
    await using credentials = await TestMailServer.listen(world.server, {
        secureTransport: "off",
    });
    await using sender = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        plain: PLAIN,
        answers: { "MAIL FROM:<notices@destack.app>": "553 5.7.1 sender not allowed" },
    });
    const failures = await Promise.all(
        [greeting, credentials, sender].map((server) =>
            readFailure(createClient(world, server).submit(ENVELOPE, message)),
        ),
    );

    expect(failures).toEqual([
        {
            refusal: [
                "UNPROCESSABLE_CONTENT",
                "COMMAND_REJECTED",
                "server responded to greeting with 554 service unavailable",
            ],
            reply: { code: 554, enhancedCode: "5.3.2", text: "service unavailable" },
        },
        {
            refusal: [
                "UNPROCESSABLE_CONTENT",
                "COMMAND_REJECTED",
                "server responded to AUTH PLAIN with 535 authentication failed",
            ],
            reply: { code: 535, enhancedCode: "5.7.8", text: "authentication failed" },
        },
        {
            refusal: [
                "UNPROCESSABLE_CONTENT",
                "COMMAND_REJECTED",
                "server responded to MAIL FROM with 553 sender not allowed",
            ],
            reply: { code: 553, enhancedCode: "5.7.1", text: "sender not allowed" },
        },
    ]);
});

test("fail on malformed, mismatched and overlong replies", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using malformed = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        answers: { greeting: "hello" },
    });
    await using mismatched = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        answers: { greeting: "220-mail.test ESMTP\n250 ready" },
    });
    await using overlong = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        answers: { greeting: `220 ${"x".repeat(600)}` },
    });
    await using unended = await listenRaw(world, (_socket, send) => {
        void send(`220 ${"x".repeat(600)}`);
    });
    const failures = [
        await readFailure(createClient(world, malformed).submit(ENVELOPE, message)),
        await readFailure(createClient(world, mismatched).submit(ENVELOPE, message)),
        await readFailure(createClient(world, overlong).submit(ENVELOPE, message)),
        await readFailure(createClient(world, unended).submit(ENVELOPE, message)),
    ];

    expect(failures).toEqual([
        {
            refusal: [
                "BAD_GATEWAY",
                "RESPONSE_INVALID",
                'server sent "hello", which is not a reply line',
            ],
            reply: undefined,
        },
        {
            refusal: [
                "BAD_GATEWAY",
                "RESPONSE_INVALID",
                "server continued a 220 reply with a 250 line",
            ],
            reply: undefined,
        },
        {
            refusal: [
                "BAD_GATEWAY",
                "RESPONSE_INVALID",
                "server sent a reply line longer than 512 octets",
            ],
            reply: undefined,
        },
        {
            refusal: [
                "BAD_GATEWAY",
                "RESPONSE_INVALID",
                "server sent a reply line longer than 512 octets",
            ],
            reply: undefined,
        },
    ]);
});

test("fail on a greeting silent or trickling past RFC 5321's five minutes, a token stalled past five minutes and a refused connection", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    await using silent = await listenRaw(world, () => undefined);
    await using trickling = await listenRaw(world, (socket, send) => {
        // trickle an endless greeting
        const leaving = new AbortController();
        void socket.closed.then(() => leaving.abort());
        const trickle = async () => {
            await send("220 ");
            for (;;) {
                await world.clock.sleep(TRICKLE_MILLISECONDS, leaving.signal);
                await send("x");
            }
        };
        trickle().catch(() => undefined);
    });
    await using stalled = await TestMailServer.listen(world.server, {
        secureTransport: "off",
        xoauth2: XOAUTH2,
    });
    const closed = await listenRaw(world, () => undefined);
    await closed.close();
    const authentication: Authentication = {
        kind: "xoauth2",
        username: XOAUTH2.username,
        token: () => new Promise(() => {}),
    };
    const failures = [
        await readFailure(createClient(world, silent).submit(ENVELOPE, message)),
        await readFailure(createClient(world, trickling).submit(ENVELOPE, message)),
        await readFailure(
            createClient(world, stalled, { authentication }).submit(ENVELOPE, message),
        ),
        await readFailure(createClient(world, closed).submit(ENVELOPE, message)),
    ];

    expect(failures).toEqual([
        {
            refusal: [
                "GATEWAY_TIMEOUT",
                "PROVIDER_TIMEOUT",
                `127.0.0.1:${silent.port} timed out after 300000 ms`,
            ],
            reply: undefined,
        },
        {
            refusal: [
                "GATEWAY_TIMEOUT",
                "PROVIDER_TIMEOUT",
                `127.0.0.1:${trickling.port} timed out after 300000 ms`,
            ],
            reply: undefined,
        },
        {
            refusal: [
                "GATEWAY_TIMEOUT",
                "PROVIDER_TIMEOUT",
                `127.0.0.1:${stalled.port} timed out after 300000 ms`,
            ],
            reply: undefined,
        },
        {
            refusal: [
                "BAD_GATEWAY",
                "PROVIDER_UNREACHABLE",
                `connection to 127.0.0.1:${closed.port} failed: connection refused: nothing listens at 127.0.0.1:${closed.port}`,
            ],
            reply: undefined,
        },
    ]);
});

test("fail on a server silent past RFC 5321's timeout for MAIL, RCPT, DATA and the data's end", async () => {
    // stall one step per server
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    const steps = ["MAIL FROM:<notices@destack.app>", "RCPT TO:<ada@example.com>", "DATA", "."];
    const failures: { port: number; failure: Record<string, unknown>; waited: number }[] = [];
    for (const step of steps) {
        await using server = await TestMailServer.listen(world.server, {
            secureTransport: "off",
            plain: PLAIN,
            silences: [step],
        });
        const started = world.clock.now();
        const failure = await readFailure(createClient(world, server).submit(ENVELOPE, message));
        failures.push({ port: server.port, failure, waited: world.clock.now() - started });
    }

    // time out at each step's RFC 5321 limit
    expect(failures).toEqual(
        [300_000, 300_000, 120_000, 600_000].map((timeout, index) => {
            const port = failures[index]?.port;
            const refusal = [
                "GATEWAY_TIMEOUT",
                "PROVIDER_TIMEOUT",
                `127.0.0.1:${port} timed out after ${timeout} ms`,
            ];

            return { port, failure: { refusal, reply: undefined }, waited: timeout };
        }),
    );
});

test("refuse envelope addresses that could inject commands, and empty envelopes", async () => {
    const world = simulate();
    const message = await MimeMessage.compose(DRAFT, world.environment.crypto);
    const client = createClient(world, { port: 25 });
    const failures = await Promise.all([
        readFailure(client.submit({ ...ENVELOPE, sender: "a@b.example>\r\nRSET" }, message)),
        readFailure(
            client.submit(
                { ...ENVELOPE, recipients: ["ada@example.com", "<eve@example.com>"] },
                message,
            ),
        ),
        readFailure(client.submit({ ...ENVELOPE, recipients: [] }, message)),
    ]);

    expect(failures).toEqual([
        {
            refusal: [
                "BAD_REQUEST",
                "ENVELOPE_INVALID",
                'address "a@b.example>\\r\\nRSET" is not a dot-atom addr-spec',
            ],
            reply: undefined,
        },
        {
            refusal: [
                "BAD_REQUEST",
                "ENVELOPE_INVALID",
                'address "<eve@example.com>" is not a dot-atom addr-spec',
            ],
            reply: undefined,
        },
        {
            refusal: ["BAD_REQUEST", "ENVELOPE_INVALID", "envelope has no recipients"],
            reply: undefined,
        },
    ]);
});

/** Open a simulated world with a mail server's node and a client's. */
function simulate(): World {
    const simulation = new Simulation({ seed: 1, start: START });

    return {
        simulation,
        clock: simulation.clock,
        server: simulation.network.server("mail"),
        environment: simulation.environment("client"),
    };
}

/** Create a client for a server on the world's loopback, submitting as the world runs. */
function createClient(
    world: World,
    server: { readonly port: number },
    options: Partial<SmtpOptions> = {},
): { readonly submit: (envelope: Envelope, submitted: MimeMessage) => Promise<Submission> } {
    // create the client
    const client = new SmtpClient({
        host: "127.0.0.1",
        port: server.port,
        secureTransport: "off",
        authentication: { kind: "plain", ...PLAIN },
        helo: "client.test",
        environment: world.environment,
        ...options,
    });

    // run the world until the submission settles
    return {
        submit: async (envelope, submitted) => {
            const submitting = client.submit(envelope, submitted);
            let isSettled = false;
            submitting.then(
                () => {
                    isSettled = true;
                },
                () => {
                    isSettled = true;
                },
            );
            await world.simulation.until(() => isSettled);

            return submitting;
        },
    };
}

/** Build a positive completion reply. */
function accepted(enhancedCode: string, text: string) {
    return { code: 250, enhancedCode, text };
}

/** Listen on the world's loopback, serving each connection's raw socket. */
async function listenRaw(
    world: World,
    serve: (socket: Socket, send: (text: string) => Promise<void>) => void,
): Promise<{ readonly port: number; readonly close: () => Promise<void> } & AsyncDisposable> {
    // accept connections
    const listener = await world.server.socket({
        hostname: "127.0.0.1",
        accept: (socket) => {
            const writer = socket.writable.getWriter();
            serve(socket, (text) => writer.write(new TextEncoder().encode(text)));
        },
    });
    const close = () => listener.stop();
    const { address } = listener;
    if ("unix" in address) {
        throw new TypeError("the mail server listens at a Unix socket, not a port");
    }

    return { port: address.port, close, [Symbol.asyncDispose]: close };
}

/** Read the refusal and the rejecting reply a submission fails with. */
async function readFailure(submitted: Promise<unknown>): Promise<Record<string, unknown>> {
    try {
        await submitted;
    } catch (error) {
        if (MessageError.is(error)) {
            return { refusal: [error.code, error.reason, error.message], reply: error.data };
        }
        throw error;
    }
    throw new Error("submission succeeded");
}
