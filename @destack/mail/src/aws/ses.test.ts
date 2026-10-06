import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import type { Mail } from "../mail/index.ts";
import { MimeMessage } from "../mime/index.ts";
import { SesClient } from "./client.ts";
import { SesTransport } from "./ses.ts";

/** The SESv2 SendEmail body the transport posts. */
const SendEmail = schema.object({
    FromEmailAddress: schema.string(),
    Destination: schema.object({ ToAddresses: schema.array(schema.string()) }),
    Content: schema.object({ Raw: schema.object({ Data: schema.string() }) }),
});

/** The email every test sends. */
const MAIL: Mail = {
    to: "ada@example.com",
    subject: "Alice mentioned you",
    text: "On it",
    key: "call-01",
};

/** A transport over a fake SES answering every send with one response, recording the bodies. */
function transport(answer: () => Response): {
    readonly mail: SesTransport;
    readonly bodies: string[];
} {
    const bodies: string[] = [];
    const client = new SesClient({
        region: "eu-central-1",
        credentials: async () => ({ accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" }),
        fetch: async (_, initialize) => {
            bodies.push(typeof initialize.body === "string" ? initialize.body : "");

            return answer();
        },
    });

    return { mail: new SesTransport(client, { from: "Destack <notices@destack.app>" }), bodies };
}

/** Answer as SES refuses: a status and an error type header. */
function refused(status: number, awsCode: string): () => Response {
    return () =>
        Response.json(
            { message: `refused with ${awsCode}` },
            { status, headers: { "x-amzn-ErrorType": awsCode } },
        );
}

test("send the composed message from the sender to the address, its Message-ID digesting the key", async () => {
    const { mail, bodies } = transport(() => Response.json({ MessageId: "0102018f-message" }));

    expect(await mail.send(MAIL)).toEqual({ outcome: "sent" });
    const request = readSend(bodies[0]);
    const content = atob(request.data);
    const date = /\r\nDate: ([^\r]+)\r\n/u.exec(content)?.[1] ?? "";
    const expected = await MimeMessage.compose({
        from: "Destack <notices@destack.app>",
        to: ["ada@example.com"],
        subject: MAIL.subject,
        date: new Date(date),
        key: MAIL.key,
        text: MAIL.text,
    });
    expect([request.from, request.to, content]).toEqual([
        "notices@destack.app",
        ["ada@example.com"],
        expected.content,
    ]);

    // compose the same Message-ID for the same key
    await mail.send(MAIL);
    const again = atob(readSend(bodies[1]).data);
    expect(messageId(again)).toBe(expected.messageId);
});

test.each([
    ["throttling", 429, "TooManyRequestsException", "retry"],
    ["an outage", 503, "ServiceUnavailable", "retry"],
    ["a rejected message", 400, "MessageRejected", "failed"],
    ["a suspended account", 400, "AccountSuspendedException", "failed"],
    ["an unverified MAIL FROM domain", 400, "MailFromDomainNotVerifiedException", "failed"],
] as const)("read %s as an outcome with its AWS code", async (_, status, code, outcome) => {
    const { mail } = transport(refused(status, code));

    expect(await mail.send(MAIL)).toEqual({
        outcome,
        error: {
            code,
            message: `SES refused the send with HTTP ${status} ${code}: refused with ${code}`,
        },
    });
});

test("fail an address the message cannot carry without asking SES", async () => {
    const { mail, bodies } = transport(() => Response.json({ MessageId: "0102018f-message" }));

    expect(await mail.send({ ...MAIL, to: "not an address" })).toEqual({
        outcome: "failed",
        error: {
            code: "INVALID_ADDRESS",
            message: 'address "not an address" is not a dot-atom addr-spec',
        },
    });
    expect(bodies).toEqual([]);
});

/** Read the sender, recipients and raw data of a SendEmail body. */
function readSend(body: string | undefined): {
    readonly from: string;
    readonly to: readonly string[];
    readonly data: string;
} {
    const request = SendEmail.parse(JSON.parse(body ?? "null"));

    return {
        from: request.FromEmailAddress,
        to: request.Destination.ToAddresses,
        data: request.Content.Raw.Data,
    };
}

/** Read the Message-ID header of a message. */
function messageId(content: string): string | undefined {
    return /\r\nMessage-ID: (\S+)\r\n/u.exec(content)?.[1];
}
