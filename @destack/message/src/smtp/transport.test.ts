import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import type { Mail } from "../email/index.ts";
import { MimeMessage } from "../mime/index.ts";
import type { Outcome } from "../provider/index.ts";
import { TestMailServer } from "../test/index.ts";
import { Simulation } from "@destack/environment/simulation";
import { SmtpClient } from "./client.ts";
import { SmtpTransport } from "./transport.ts";

/** The PLAIN credentials the test servers accept. */
const PLAIN = { username: "ada", password: "secret" };

/** The sender's mailbox. */
const FROM = "Destack <notices@destack.app>";

/** The email every test sends. */
const MAIL: Mail = {
    to: "ada@example.com",
    subject: "New sign-in",
    text: "A new device signed in.",
    key: "security-notice-01",
    date: Date.UTC(2026, 9, 5),
};

/** The time every simulation starts at. */
const START = Date.UTC(2026, 9, 5);

/** A simulation with a mail server's server and a client's environment. */
function simulate() {
    const simulation = new Simulation({ seed: 1, start: START });

    return { simulation, serve: simulation.network.server("mail") };
}

/** Send an email through a transport to a test server's port, as the world runs. */
async function send(
    world: ReturnType<typeof simulate>,
    port: number,
    mail: Mail,
): Promise<Outcome> {
    // submit with PLAIN in plain text on the loopback
    const client = new SmtpClient({
        host: "127.0.0.1",
        port,
        secureTransport: "off",
        authentication: { kind: "plain", ...PLAIN },
        helo: "client.test",
        environment: world.simulation.environment("client"),
    });
    const sending = new SmtpTransport(client, { from: FROM }).send(mail);

    // run the world until the outcome is read
    let isSettled = false;
    void sending.finally(() => {
        isSettled = true;
    });
    await world.simulation.until(() => isSettled);

    return sending;
}

test("submit the composed message from the sender to the address, its Message-ID digesting the key", async () => {
    const { crypto } = testSimulation().environment("message");

    const world = simulate();
    await using server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const expected = await MimeMessage.compose(
        {
            from: FROM,
            to: [MAIL.to],
            subject: MAIL.subject,
            date: new Date(MAIL.date),
            key: MAIL.key,
            text: MAIL.text,
        },
        crypto,
    );

    // send it with the sender's address and the recipient in the envelope
    expect(await send(world, server.port, MAIL)).toEqual({ kind: "sent" });
    expect([server.messages, server.transcripts]).toEqual([
        [expected.content],
        [
            [
                "S: 220 mail.test ESMTP ready",
                "C: EHLO client.test",
                "S: 250-mail.test greets client.test",
                "S: 250-AUTH PLAIN XOAUTH2",
                "S: 250 ENHANCEDSTATUSCODES",
                "C: AUTH PLAIN AGFkYQBzZWNyZXQ=",
                "S: 235 2.7.0 authenticated",
                "C: MAIL FROM:<notices@destack.app>",
                "S: 250 2.1.0 sender ok",
                "C: RCPT TO:<ada@example.com>",
                "S: 250 2.1.5 recipient ok",
                "C: DATA",
                "S: 354 end data with <CR><LF>.<CR><LF>",
                "C: .",
                "S: 250 2.0.0 queued",
                "C: QUIT",
                "S: 221 2.0.0 bye",
            ],
        ],
    ]);
});

test.each<[string, Readonly<Record<string, string>>, Outcome]>([
    [
        "a busy mailbox",
        { "RCPT TO:<ada@example.com>": "450 4.2.1 mailbox busy" },
        {
            kind: "deferred",
            refusal: { reason: "COMMAND_DEFERRED", code: "450", message: "mailbox busy" },
        },
    ],
    [
        "an unknown mailbox",
        { "RCPT TO:<ada@example.com>": "550 5.1.1 no such user" },
        {
            kind: "failed",
            refusal: { reason: "COMMAND_REJECTED", code: "550", message: "no such user" },
        },
    ],
    [
        "a full disk at data",
        { DATA: "452 4.3.1 insufficient system storage" },
        {
            kind: "deferred",
            refusal: {
                reason: "COMMAND_DEFERRED",
                code: "452",
                message: "insufficient system storage",
            },
        },
    ],
    [
        "rejected content",
        { ".": "554 5.7.1 message content rejected" },
        {
            kind: "failed",
            refusal: {
                reason: "COMMAND_REJECTED",
                code: "554",
                message: "message content rejected",
            },
        },
    ],
    [
        "a closing server",
        { greeting: "421 4.3.2 service shutting down" },
        {
            kind: "deferred",
            refusal: { reason: "COMMAND_DEFERRED", code: "421", message: "service shutting down" },
        },
    ],
    [
        "refused credentials",
        { "AUTH PLAIN AGFkYQBzZWNyZXQ=": "535 5.7.8 bad credentials" },
        {
            kind: "failed",
            refusal: { reason: "COMMAND_REJECTED", code: "535", message: "bad credentials" },
        },
    ],
])("read %s as an outcome with the reply", async (_, answers, outcome) => {
    const world = simulate();
    await using server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
        answers,
    });

    expect(await send(world, server.port, MAIL)).toEqual(outcome);
});

test("retry a server it cannot connect to", async () => {
    const world = simulate();
    const server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const port = server.port;
    await server.close();

    expect(await send(world, port, MAIL)).toEqual({
        kind: "deferred",
        refusal: {
            reason: "PROVIDER_UNREACHABLE",
            message: `connection to 127.0.0.1:${port} failed: connection refused: nothing listens at 127.0.0.1:${port}`,
        },
    });
});

test("fail an address the message cannot carry without connecting", async () => {
    const world = simulate();
    await using server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
    });

    expect(await send(world, server.port, { ...MAIL, to: "not an address" })).toEqual({
        kind: "failed",
        refusal: {
            reason: "ADDRESS_INVALID",
            message: 'address "not an address" is not a dot-atom addr-spec',
        },
    });
    expect(server.transcripts).toEqual([]);
});
