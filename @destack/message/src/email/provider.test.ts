import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MailError } from "@destack/mail/error";
import { type TestMailAnswer, TestMailTransport } from "@destack/mail/test";
import type { Outcome } from "../provider/index.ts";
import { TestMessage } from "../test/index.ts";
import { emailProvider } from "./provider.ts";
import { testSimulation } from "@destack/object/test";

/** The identifier of the message every test sends, which keys its email. */
const ID = schema.identifier("message").parse("message-019f5530-8000-7000-8000-0000000000e1");

/** The time every test's message was created at. */
const NOW = Date.UTC(2026, 9, 5);

/** The space every test sends from. */
const SCOPE = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000e2");

/** The notification every test's message is about. */
const SOURCE = {
    packageId: PackageId.parse("package-019f5530-8000-7000-8000-0000000000e3"),
    type: "notification",
    scope: SCOPE,
    id: "notification-019f5530-8000-7000-8000-0000000000e4",
};

/** Send one email message through a transport answering as scripted, reading the outcome. */
function send(transport: TestMailTransport): Promise<Outcome> {
    return emailProvider(transport).send(
        {
            id: ID,
            scope: SCOPE,
            createdAt: NOW,
            to: { channel: "email", address: "ada@example.com" },
            source: SOURCE,
            content: {
                channel: "email",
                subject: "Mentioned",
                text: "On it",
                html: "<p>On it</p>",
            },
        },
        TestMessage.store(testSimulation(NOW).environment("email")),
    );
}

test("send an email message through the transport, keyed by its identifier with its HTML body", async () => {
    const transport = new TestMailTransport();

    // send one email message on the email channel
    const outcome = await send(transport);
    expect({ outcome, sent: transport.sent }).toEqual({
        outcome: { kind: "sent" },
        sent: [
            {
                to: "ada@example.com",
                subject: "Mentioned",
                text: "On it",
                html: "<p>On it</p>",
                key: ID,
                date: NOW,
            },
        ],
    });
});

test.each<[TestMailAnswer, Outcome]>([
    [
        MailError.of(
            "COMMAND_UNAVAILABLE",
            "command 'RCPT TO' is unavailable: the server answered 450 'mailbox busy'",
            {
                data: { code: 450, enhancedCode: "4.2.1", text: "mailbox busy" },
            },
        ),
        {
            kind: "deferred",
            refusal: {
                reason: "COMMAND_UNAVAILABLE",
                code: "450",
                message: "command 'RCPT TO' is unavailable: the server answered 450 'mailbox busy'",
            },
        },
    ],
    [
        MailError.of(
            "COMMAND_INVALID",
            "command 'RCPT TO' is invalid: the server answered 550 'no such user'",
            {
                data: { code: 550, enhancedCode: "5.1.1", text: "no such user" },
            },
        ),
        {
            kind: "failed",
            refusal: {
                reason: "COMMAND_INVALID",
                code: "550",
                message: "command 'RCPT TO' is invalid: the server answered 550 'no such user'",
            },
        },
    ],
    [
        MailError.of("PROVIDER_EXHAUSTED", "sending rate exceeds the SES limit", {
            data: { awsCode: "Throttling" },
        }),
        {
            kind: "deferred",
            refusal: {
                reason: "PROVIDER_EXHAUSTED",
                code: "Throttling",
                message: "sending rate exceeds the SES limit",
            },
        },
    ],
    [
        MailError.of("CREDENTIALS_UNAVAILABLE", "SES credential is unavailable: it expired", {
            data: { awsCode: "ExpiredTokenException" },
        }),
        {
            kind: "deferred",
            refusal: {
                reason: "CREDENTIALS_UNAVAILABLE",
                code: "ExpiredTokenException",
                message: "SES credential is unavailable: it expired",
            },
        },
    ],
    [
        MailError.of("PROVIDER_UNAVAILABLE", "SES is unavailable", { data: {} }),
        {
            kind: "deferred",
            refusal: { reason: "PROVIDER_UNAVAILABLE", message: "SES is unavailable" },
        },
    ],
    [
        MailError.of("PROVIDER_UNREACHABLE", "could not connect to SES"),
        {
            kind: "deferred",
            refusal: { reason: "PROVIDER_UNREACHABLE", message: "could not connect to SES" },
        },
    ],
    [
        MailError.of("MESSAGE_INVALID", "sender address is invalid: it is unverified", {
            data: { awsCode: "MessageRejected" },
        }),
        {
            kind: "failed",
            refusal: {
                reason: "MESSAGE_INVALID",
                code: "MessageRejected",
                message: "sender address is invalid: it is unverified",
            },
        },
    ],
    [
        MailError.of("ADDRESS_INVALID", "address is invalid: it is not a dot-atom addr-spec"),
        {
            kind: "failed",
            refusal: {
                reason: "ADDRESS_INVALID",
                message: "address is invalid: it is not a dot-atom addr-spec",
            },
        },
    ],
])("refuse an email message as its transport refuses it: %#", async (answer, outcome) => {
    expect(await send(new TestMailTransport([answer]))).toEqual(outcome);
});
