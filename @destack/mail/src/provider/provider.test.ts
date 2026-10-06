import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MailFixture } from "../test/index.ts";
import { mailProvider } from "./provider.ts";

/** The identifier of the message every test sends, which keys its email. */
const ID = schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000e1");

/** The space every test sends from. */
const SCOPE = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000e2");

test("send an email message through the transport, keyed by its identifier with its HTML body", async () => {
    const transport = new MailFixture();
    const provider = mailProvider(transport);

    // send one email message on the email channel
    const outcome = await provider.send({
        id: ID,
        scope: SCOPE,
        createdAt: 0,
        to: { channel: "email", address: "ada@example.com" },
        content: {
            channel: "email",
            subject: "Mentioned",
            text: "On it",
            html: "<p>On it</p>",
        },
        secret: null,
        reader: null,
        headers: null,
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
