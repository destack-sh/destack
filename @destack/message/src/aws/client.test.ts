import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { MimeMessage, type Envelope } from "../mime/index.ts";
import { AwsSigner } from "@destack/identity/aws";
import type { Fetch } from "@destack/object";
import { MessageError, type MessageReason, SesFailure } from "../error/index.ts";
import { Outcome } from "../provider/index.ts";
import { SesClient, type SesOptions } from "./index.ts";

/** The WebCrypto the clients sign with, a world's. */
/** The simulated node SES clients run on. */
const node = testSimulation().environment("ses");

/** The environment of a client sending through a fetch on the simulated node's clock. */
function environment(fetch: Fetch): SesOptions["environment"] {
    return { clock: node.clock, crypto: node.crypto, network: { ...node.network, fetch } };
}

/** The credentials every client signs with. */
const CREDENTIALS = {
    accessKeyId: "AKIDEXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
    sessionToken: "session-token",
};

/** The time the requests are signed at. */
const SENT_AT = new Date("2026-10-03T10:00:00Z");

/** The envelope every test sends. */
const ENVELOPE: Envelope = { sender: "notices@destack.app", recipients: ["ada@example.com"] };

/** The message every test sends. */
const MESSAGE = await MimeMessage.compose(
    {
        from: "Destack <notices@destack.app>",
        to: ["ada@example.com"],
        subject: "Sign in to Destack",
        date: new Date("2026-10-03T10:00:00Z"),
        key: "sign-in-01",
        text: "Sign in with this link.",
    },
    node.crypto,
);

/** One request a fake SES received. */
interface Received {
    /** The URL. */
    readonly url: string;
    /** The method, headers and body. */
    readonly initialize: RequestInit;
}

/** The refusals of SES: each case's answer, failure code, AWS code and whether a retry may pass. */
const REFUSALS: [string, () => Response, MessageReason, string | undefined, boolean][] = [
    [
        "throttling",
        () => refused(429, "TooManyRequestsException", "Maximum sending rate exceeded."),
        "PROVIDER_THROTTLED",
        "TooManyRequestsException",
        true,
    ],
    [
        "an exceeded quota",
        () => refused(400, "LimitExceededException", "Daily message quota exceeded."),
        "PROVIDER_THROTTLED",
        "LimitExceededException",
        true,
    ],
    [
        "an internal failure",
        () => refused(500, "InternalFailure", "Internal error."),
        "PROVIDER_UNAVAILABLE",
        "InternalFailure",
        true,
    ],
    [
        "an unavailable service",
        () => new Response("Service Unavailable", { status: 503 }),
        "PROVIDER_UNAVAILABLE",
        undefined,
        true,
    ],
    [
        "expired credentials",
        () =>
            refused(
                403,
                "ExpiredTokenException",
                "The security token included in the request is expired",
            ),
        "CREDENTIALS_EXPIRED",
        "ExpiredTokenException",
        true,
    ],
    [
        "a rejected message",
        () => refused(400, "MessageRejected", "Email address is not verified."),
        "MESSAGE_REJECTED",
        "MessageRejected",
        false,
    ],
    [
        "an unverified MAIL FROM domain",
        () =>
            refused(400, "MailFromDomainNotVerifiedException", "MAIL FROM domain is not verified."),
        "SENDER_UNVERIFIED",
        "MailFromDomainNotVerifiedException",
        false,
    ],
    [
        "a suspended account",
        () => refused(400, "AccountSuspendedException", "Account suspended."),
        "SENDING_SUSPENDED",
        "AccountSuspendedException",
        false,
    ],
    [
        "paused sending",
        () => refused(400, "SendingPausedException", "Sending paused."),
        "SENDING_SUSPENDED",
        "SendingPausedException",
        false,
    ],
    [
        "a bad signature",
        () => refused(403, "InvalidSignatureException", "Signature mismatch."),
        "CREDENTIALS_REFUSED",
        "InvalidSignatureException",
        false,
    ],
    [
        "an unknown configuration set",
        () => refused(404, "NotFoundException", "Configuration set does not exist."),
        "REQUEST_INVALID",
        "NotFoundException",
        false,
    ],
    [
        "a body-typed bad request",
        () =>
            Response.json(
                { __type: "com.amazonaws.ses#BadRequestException", message: "Bad request." },
                { status: 400 },
            ),
        "REQUEST_INVALID",
        "BadRequestException",
        false,
    ],
    [
        "an answer without a MessageId",
        () => Response.json({}),
        "RESPONSE_INVALID",
        undefined,
        false,
    ],
];

/** A fake SES answering every request with one response, recording what it received. */
function fakeSes(answer: () => Response | Promise<Response>): {
    readonly fetch: Fetch;
    readonly received: Received[];
} {
    const received: Received[] = [];

    return {
        fetch: async (request) => {
            received.push({
                url: request.url,
                initialize: {
                    method: request.method,
                    headers: request.headers,
                    body: await request.text(),
                },
            });

            return answer();
        },
        received,
    };
}

/** Answer as SES refuses: a status, an error type header and a JSON message. */
function refused(status: number, awsCode: string, message: string): Response {
    return new Response(JSON.stringify({ message }), {
        status,
        headers: {
            "content-type": "application/json",
            "x-amzn-ErrorType": `${awsCode}:http://internal.amazon.com/coral/com.amazon.coral.service/`,
        },
    });
}

/** Read a header a request sent. */
function header(initialize: RequestInit, name: string): string | undefined {
    return new Headers(initialize.headers).get(name) ?? undefined;
}

test("post the raw message to the region's SESv2 endpoint, signed, and return the MessageId", async () => {
    const ses = fakeSes(() => Response.json({ MessageId: "0102018f-message" }));
    const client = new SesClient({
        region: "eu-central-1",
        endpoint: SesClient.endpoint("eu-central-1"),
        credentials: async () => CREDENTIALS,
        configurationSet: "destack",
        environment: environment(ses.fetch),
    });

    // send and read the MessageId
    expect(await client.send(ENVELOPE, MESSAGE, SENT_AT)).toBe("0102018f-message");
    const [request] = ses.received;
    if (request === undefined) {
        throw new TypeError("expected one request");
    }

    // post the envelope and the base64 message to the region's host
    const body = typeof request.initialize.body === "string" ? request.initialize.body : "";
    expect([request.url, request.initialize.method, JSON.parse(body)]).toEqual([
        "https://email.eu-central-1.amazonaws.com/v2/email/outbound-emails",
        "POST",
        {
            FromEmailAddress: "notices@destack.app",
            Destination: { ToAddresses: ["ada@example.com"] },
            Content: { Raw: { Data: btoa(MESSAGE.content) } },
            ConfigurationSetName: "destack",
        },
    ]);

    // sign the host, content type, date and session token for ses in the region
    expect([
        header(request.initialize, "host"),
        header(request.initialize, "content-type"),
        header(request.initialize, "x-amz-security-token"),
        header(request.initialize, "x-amz-date"),
    ]).toEqual([
        "email.eu-central-1.amazonaws.com",
        "application/json",
        "session-token",
        "20261003T100000Z",
    ]);
    const url = new URL(request.url);
    const expected = await new AwsSigner({
        service: "ses",
        region: "eu-central-1",
        credentials: CREDENTIALS,
        crypto: node.crypto,
    }).authorize(
        {
            method: "POST",
            path: url.pathname,
            query: [],
            headers: { "content-type": "application/json", host: url.host },
            payloadHash: await AwsSigner.payloadHash(body, node.crypto),
        },
        SENT_AT,
    );
    expect(header(request.initialize, "authorization")).toBe(expected["authorization"]);
});

test("leave the configuration set out when none is set, and ask the credentials on each send", async () => {
    const ses = fakeSes(() => Response.json({ MessageId: "0102018f-message" }));
    let asked = 0;
    const client = new SesClient({
        region: "us-east-1",
        endpoint: SesClient.endpoint("us-east-1"),
        credentials: async () => {
            asked += 1;

            return { accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" };
        },
        environment: environment(ses.fetch),
    });
    await client.send(ENVELOPE, MESSAGE, SENT_AT);
    await client.send(ENVELOPE, MESSAGE, SENT_AT);

    // post only the envelope and message, unsigned by any session token
    const [request] = ses.received;
    const body = typeof request?.initialize.body === "string" ? request.initialize.body : "";
    expect([
        asked,
        JSON.parse(body),
        header(request?.initialize ?? {}, "x-amz-security-token"),
    ]).toEqual([
        2,
        {
            FromEmailAddress: "notices@destack.app",
            Destination: { ToAddresses: ["ada@example.com"] },
            Content: { Raw: { Data: btoa(MESSAGE.content) } },
        },
        undefined,
    ]);
});

test("refuse a malformed region", async () => {
    const create = () =>
        new SesClient({
            region: "eu central",
            endpoint: SesClient.endpoint("eu central"),
            credentials: async () => CREDENTIALS,
            environment: environment(async () => new Response(null, { status: 500 })),
        });
    expect(create).toThrow(new TypeError('region "eu central" is not an AWS region name'));
});

test.each(REFUSALS)(
    "map %s to its failure with the AWS code",
    async (_, answer, code, awsCode, isRetryable) => {
        const client = new SesClient({
            region: "eu-central-1",
            endpoint: SesClient.endpoint("eu-central-1"),
            credentials: async () => CREDENTIALS,
            environment: environment(fakeSes(answer).fetch),
        });

        const error = await client
            .send(ENVELOPE, MESSAGE, SENT_AT)
            .catch((caught: unknown) => caught);
        if (!MessageError.is(error)) {
            throw new TypeError(`expected a MessageError, got ${String(error)}`);
        }
        const retried = Outcome.refuse({ reason: error.reason, message: error.message });
        expect([
            error.reason,
            SesFailure.safeParse(error.data).data?.awsCode,
            retried.kind === "deferred",
        ]).toEqual([code, awsCode, isRetryable]);
    },
);

test("map a failed connection to a retryable failure carrying its cause", async () => {
    const cause = new TypeError("fetch failed");
    const client = new SesClient({
        region: "eu-central-1",
        endpoint: SesClient.endpoint("eu-central-1"),
        credentials: async () => CREDENTIALS,
        environment: environment(async () => {
            throw cause;
        }),
    });

    const error = await client.send(ENVELOPE, MESSAGE, SENT_AT).catch((caught: unknown) => caught);
    if (!MessageError.is(error)) {
        throw new TypeError(`expected a MessageError, got ${String(error)}`);
    }
    expect([error.reason, error.cause]).toEqual(["PROVIDER_UNREACHABLE", cause]);
});
