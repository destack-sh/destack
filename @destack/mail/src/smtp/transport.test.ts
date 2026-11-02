import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import type { Mail } from "../transport/index.ts";
import { MimeMessage } from "../mime/index.ts";
import { MailError, type MailReason, type SmtpReply } from "../error/index.ts";
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

/** Send an email to a test server's port as the world runs, reading its refusal. */
async function send(world: ReturnType<typeof simulate>, port: number, mail: Mail) {
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

    // run the world until the send settles
    let isSettled = false;
    const settled = sending.then(
        () => "sent" as const,
        (error: unknown) => error,
    );
    void settled.finally(() => {
        isSettled = true;
    });
    await world.simulation.waitUntil(() => isSettled);

    // read a mail failure by its reason, data and message
    const error = await settled;
    if (error === "sent") {
        return error;
    } else if (!MailError.is(error)) {
        throw error;
    }

    return { reason: error.reason, data: error.data, message: error.message };
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
    expect(await send(world, server.port, MAIL)).toBe("sent");
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

test.each<[string, Readonly<Record<string, string>>, MailReason, SmtpReply]>([
    [
        "a busy mailbox",
        { "RCPT TO:<ada@example.com>": "450 4.2.1 mailbox busy" },
        "COMMAND_DEFERRED",
        { code: 450, enhancedCode: "4.2.1", text: "mailbox busy" },
    ],
    [
        "an unknown mailbox",
        { "RCPT TO:<ada@example.com>": "550 5.1.1 no such user" },
        "COMMAND_REJECTED",
        { code: 550, enhancedCode: "5.1.1", text: "no such user" },
    ],
    [
        "a full disk at data",
        { DATA: "452 4.3.1 insufficient system storage" },
        "COMMAND_DEFERRED",
        { code: 452, enhancedCode: "4.3.1", text: "insufficient system storage" },
    ],
    [
        "rejected content",
        { ".": "554 5.7.1 message content rejected" },
        "COMMAND_REJECTED",
        { code: 554, enhancedCode: "5.7.1", text: "message content rejected" },
    ],
    [
        "a closing server",
        { greeting: "421 4.3.2 service shutting down" },
        "COMMAND_DEFERRED",
        { code: 421, enhancedCode: "4.3.2", text: "service shutting down" },
    ],
    [
        "refused credentials",
        { "AUTH PLAIN AGFkYQBzZWNyZXQ=": "535 5.7.8 bad credentials" },
        "COMMAND_REJECTED",
        { code: 535, enhancedCode: "5.7.8", text: "bad credentials" },
    ],
])("refuse %s by the reply's class", async (_, answers, reason, reply) => {
    const world = simulate();
    await using server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
        answers,
    });

    expect(await send(world, server.port, MAIL)).toEqual({
        reason,
        data: reply,
        message: reply.text,
    });
});

test("refuse a server it cannot connect to as unreachable", async () => {
    const world = simulate();
    const server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
    });
    const port = server.port;
    await server.close();

    expect(await send(world, port, MAIL)).toEqual({
        reason: "PROVIDER_UNREACHABLE",
        data: undefined,
        message: `connection to 127.0.0.1:${port} failed: connection refused: nothing listens at 127.0.0.1:${port}`,
    });
});

test("refuse an address the message cannot carry without connecting", async () => {
    const world = simulate();
    await using server = await TestMailServer.listen(world.serve, {
        secureTransport: "off",
        plain: PLAIN,
    });

    expect(await send(world, server.port, { ...MAIL, to: "not an address" })).toEqual({
        reason: "ADDRESS_INVALID",
        data: undefined,
        message: 'address "not an address" is not a dot-atom addr-spec',
    });
    expect(server.transcripts).toEqual([]);
});
