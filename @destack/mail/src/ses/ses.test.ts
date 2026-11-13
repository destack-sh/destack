import { schema } from "@destack/schema";
import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import type { Mail } from "../transport/index.ts";
import { MimeMessage } from "../mime/index.ts";
import type { Network } from "@destack/environment";
import { MailError } from "../error/index.ts";
import { SesClient, type SesOptions } from "./client.ts";
import { SesTransport } from "./ses.ts";

/** The simulated node SES clients run on. */
const node = testSimulation().environment("ses");

/** The environment of a client sending through a fetch on the simulated node's clock. */
function environment(fetch: Network["fetch"]): SesOptions["environment"] {
    return {
        clock: node.clock,
        crypto: node.crypto,
        network: { ...node.network, fetch },
        telemetry: node.telemetry,
    };
}

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
    date: Date.UTC(2026, 9, 5),
};

/** A transport over a fake SES answering every send with one response, recording the bodies. */
function transport(answer: () => Response): {
    readonly mail: SesTransport;
    readonly bodies: string[];
} {
    const bodies: string[] = [];
    const client = new SesClient({
        region: "eu-central-1",
        endpoint: SesClient.endpoint("eu-central-1"),
        credentials: async () => ({ accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" }),
        environment: environment(async (request) => {
            bodies.push(await request.text());

            return answer();
        }),
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

    await mail.send(MAIL);
    const request = readSend(bodies[0]);
    const content = atob(request.data);
    const expected = await MimeMessage.compose(
        {
            from: "Destack <notices@destack.app>",
            to: ["ada@example.com"],
            subject: MAIL.subject,
            date: new Date(MAIL.date),
            key: MAIL.key,
            text: MAIL.text,
        },
        node.crypto,
    );
    expect([request.from, request.to, content]).toEqual([
        "notices@destack.app",
        ["ada@example.com"],
        expected.content,
    ]);

    // compose the same message, Message-ID included, for the same key
    await mail.send(MAIL);
    expect(atob(readSend(bodies[1]).data)).toBe(expected.content);
});

test.each([
    ["throttling", 429, "TooManyRequestsException", "PROVIDER_EXHAUSTED"],
    ["an outage", 503, "ServiceUnavailable", "PROVIDER_UNAVAILABLE"],
    ["a rejected message", 400, "MessageRejected", "MESSAGE_INVALID"],
    ["a suspended account", 400, "AccountSuspendedException", "SENDING_UNAVAILABLE"],
    ["an unverified MAIL FROM domain", 400, "MailFromDomainNotVerifiedException", "SENDER_INVALID"],
] as const)("refuse %s with its reason and AWS code", async (_, status, awsCode, reason) => {
    const { mail } = transport(refused(status, awsCode));

    expect(await refusal(mail.send(MAIL))).toEqual({
        reason,
        data: { awsCode },
        message: `SES refused the send with HTTP ${status} ${awsCode}: refused with ${awsCode}`,
    });
});

test("fail an address the message cannot carry without asking SES", async () => {
    const { mail, bodies } = transport(() => Response.json({ MessageId: "0102018f-message" }));

    expect(await refusal(mail.send({ ...MAIL, to: "not an address" }))).toEqual({
        reason: "ADDRESS_INVALID",
        data: undefined,
        message: `address "not an address" is invalid: it is not a dot-atom addr-spec`,
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

/** Read the mail failure a send rejects with: its reason, data and message. */
async function refusal(sending: Promise<void>) {
    const error = await sending.then(
        () => undefined,
        (caught: unknown) => caught,
    );
    if (!MailError.is(error)) {
        throw new TypeError(`expected a MailError, got '${String(error)}'`);
    }

    return { reason: error.reason, data: error.data, message: error.message };
}
