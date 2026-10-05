import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MailFixture } from "../test/index.ts";
import { mailProvider } from "./provider.ts";

/** The identifier of the message every test sends, which keys its email. */
const ID = schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000e1");

test("send an email message through the transport, keyed by its identifier with its HTML body", async () => {
    const transport = new MailFixture();
    const provider = mailProvider(transport);

    // send one email message on the email channel
    const outcome = await provider.send({
        id: ID,
        createdAt: 0,
        to: { channel: "email", address: "ada@example.com" },
        content: {
            channel: "email",
            subject: "Mentioned",
            text: "On it",
            html: "<p>On it</p>",
        },
        secret: null,
    });
    expect({ channel: provider.channel, outcome, sent: transport.sent }).toEqual({
        channel: "email",
        outcome: { outcome: "sent" },
        sent: [
            {
                to: "ada@example.com",
                subject: "Mentioned",
                text: "On it",
                html: "<p>On it</p>",
                key: ID,
            },
        ],
    });
});

test("refuse a message on another channel", async () => {
    const provider = mailProvider(new MailFixture());

    await expect(
        provider.send({
            id: ID,
            createdAt: 0,
            to: { channel: "webhook", url: "https://hooks.example.com" },
            content: { channel: "webhook", type: "issue.regressed", data: {} },
            secret: null,
        }),
    ).rejects.toThrow(`message ${ID} is no email`);
});
