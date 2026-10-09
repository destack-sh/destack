import { expect, test } from "@destack/test";
import type { Mail } from "../email/index.ts";
import { MimeMessage } from "../mime/index.ts";
import type { Outcome } from "../provider/index.ts";
import { SmtpTestServer } from "../test/index.ts";
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

/** A transport submitting to a test server on a port with its PLAIN credentials. */
function transport(port: number): SmtpTransport {
    const client = new SmtpClient({
        host: "127.0.0.1",
        port,
        security: "none",
        authentication: { kind: "plain", ...PLAIN },
        helo: "client.test",
    });

    return new SmtpTransport(client, { from: FROM });
}

test("submit the composed message from the sender to the address, its Message-ID digesting the key", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });
    const expected = await MimeMessage.compose({
        from: FROM,
        to: [MAIL.to],
        subject: MAIL.subject,
        date: new Date(MAIL.date),
        key: MAIL.key,
        text: MAIL.text,
    });

    // send it with the envelope naming the sender's address and the recipient
    expect(await transport(server.port).send(MAIL)).toEqual({ kind: "sent" });
    expect([server.messages, server.transcripts[0]?.slice(7, 10)]).toEqual([
        [expected.content],
        [
            "C: MAIL FROM:<notices@destack.app>",
            "S: 250 2.1.0 sender ok",
            "C: RCPT TO:<ada@example.com>",
        ],
    ]);
});

test.each<[string, Readonly<Record<string, string>>, Outcome]>([
    [
        "a busy mailbox",
        { "RCPT TO:<ada@example.com>": "450 4.2.1 mailbox busy" },
        { kind: "retry", error: { code: "450", message: "mailbox busy" } },
    ],
    [
        "an unknown mailbox",
        { "RCPT TO:<ada@example.com>": "550 5.1.1 no such user" },
        { kind: "failed", error: { code: "550", message: "no such user" } },
    ],
    [
        "a full disk at data",
        { DATA: "452 4.3.1 insufficient system storage" },
        { kind: "retry", error: { code: "452", message: "insufficient system storage" } },
    ],
    [
        "rejected content",
        { ".": "554 5.7.1 message content rejected" },
        { kind: "failed", error: { code: "554", message: "message content rejected" } },
    ],
    [
        "a closing server",
        { greeting: "421 4.3.2 service shutting down" },
        { kind: "retry", error: { code: "421", message: "service shutting down" } },
    ],
    [
        "refused credentials",
        { "AUTH PLAIN AGFkYQBzZWNyZXQ=": "535 5.7.8 bad credentials" },
        { kind: "failed", error: { code: "535", message: "bad credentials" } },
    ],
])("read %s as an outcome with the reply", async (_, answers, outcome) => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN, answers });

    expect(await transport(server.port).send(MAIL)).toEqual(outcome);
});

test("retry a server it cannot connect to", async () => {
    const server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });
    const port = server.port;
    await server.close();

    expect(await transport(port).send(MAIL)).toEqual({
        kind: "retry",
        error: {
            code: "CONNECTION",
            message: `connection to 127.0.0.1:${port} failed: connect ECONNREFUSED 127.0.0.1:${port}`,
        },
    });
});

test("fail an address the message cannot carry without connecting", async () => {
    await using server = await SmtpTestServer.listen({ security: "none", plain: PLAIN });

    expect(await transport(server.port).send({ ...MAIL, to: "not an address" })).toEqual({
        kind: "failed",
        error: {
            code: "INVALID_ADDRESS",
            message: 'address "not an address" is not a dot-atom addr-spec',
        },
    });
    expect(server.transcripts).toEqual([]);
});
