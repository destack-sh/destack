import { expect, test } from "@destack/test";
import { MimeMessage, type Envelope } from "../mime/index.ts";
import { awsSigner, SesError, SesClient, type SesErrorCode, type SesFetch } from "./index.ts";

/** The credentials every client signs with. */
const CREDENTIALS = {
    accessKeyId: "AKIDEXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
    sessionToken: "session-token",
};

/** The envelope every test sends. */
const ENVELOPE: Envelope = { sender: "notices@destack.app", recipients: ["ada@example.com"] };

/** The message every test sends. */
const MESSAGE = await MimeMessage.compose({
    from: "Destack <notices@destack.app>",
    to: ["ada@example.com"],
    subject: "Sign in to Destack",
    date: new Date("2026-10-03T10:00:00Z"),
    key: "sign-in-01",
    text: "Sign in with this link.",
});

/** One request a fake SES received. */
interface Received {
    /** The URL. */
    readonly url: string;
    /** The method, headers and body. */
    readonly initialize: RequestInit;
}

/** A fake SES answering every request with one response, recording what it received. */
function fakeSes(answer: () => Response | Promise<Response>): {
    readonly fetch: SesFetch;
    readonly received: Received[];
} {
    const received: Received[] = [];

    return {
        fetch: async (url, initialize) => {
            received.push({ url, initialize });

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
        credentials: async () => CREDENTIALS,
        configurationSet: "destack",
        fetch: ses.fetch,
    });

    // send and read the MessageId
    expect(await client.send(ENVELOPE, MESSAGE)).toBe("0102018f-message");
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
    const date = header(request.initialize, "x-amz-date") ?? "";
    expect([
        header(request.initialize, "host"),
        header(request.initialize, "content-type"),
        header(request.initialize, "x-amz-security-token"),
        /^\d{8}T\d{6}Z$/u.test(date),
    ]).toEqual(["email.eu-central-1.amazonaws.com", "application/json", "session-token", true]);
    const url = new URL(request.url);
    const expected = await awsSigner({
        service: "ses",
        region: "eu-central-1",
        credentials: async () => CREDENTIALS,
    }).sign(
        {
            method: "POST",
            protocol: url.protocol,
            hostname: url.hostname,
            path: url.pathname,
            query: {},
            headers: { "content-type": "application/json", host: url.host },
            body,
        },
        {
            signingDate: new Date(
                `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)}T${date.slice(9, 11)}:${date.slice(11, 13)}:${date.slice(13, 15)}Z`,
            ),
        },
    );
    expect(header(request.initialize, "authorization")).toBe(expected.headers["authorization"]);
});

test("leave the configuration set out when none is set, and ask the credentials on each send", async () => {
    const ses = fakeSes(() => Response.json({ MessageId: "0102018f-message" }));
    let asked = 0;
    const client = new SesClient({
        region: "us-east-1",
        credentials: async () => {
            asked += 1;

            return { accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" };
        },
        fetch: ses.fetch,
    });
    await client.send(ENVELOPE, MESSAGE);
    await client.send(ENVELOPE, MESSAGE);

    // post the envelope and message alone, unsigned by any session token
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

test("refuse a malformed region", () => {
    expect(
        () => new SesClient({ region: "eu central", credentials: async () => CREDENTIALS }),
    ).toThrow(new SesError("INVALID_OPTIONS", 'region "eu central" is no AWS region name'));
});

test.each<[string, () => Response, SesErrorCode, string | undefined, boolean]>([
    [
        "throttling",
        () => refused(429, "TooManyRequestsException", "Maximum sending rate exceeded."),
        "THROTTLED",
        "TooManyRequestsException",
        true,
    ],
    [
        "an exceeded quota",
        () => refused(400, "LimitExceededException", "Daily message quota exceeded."),
        "THROTTLED",
        "LimitExceededException",
        true,
    ],
    [
        "an internal failure",
        () => refused(500, "InternalFailure", "Internal error."),
        "UNAVAILABLE",
        "InternalFailure",
        true,
    ],
    [
        "an unavailable service",
        () => new Response("Service Unavailable", { status: 503 }),
        "UNAVAILABLE",
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
        "EXPIRED",
        "ExpiredTokenException",
        true,
    ],
    [
        "a rejected message",
        () => refused(400, "MessageRejected", "Email address is not verified."),
        "REJECTED",
        "MessageRejected",
        false,
    ],
    [
        "an unverified MAIL FROM domain",
        () =>
            refused(400, "MailFromDomainNotVerifiedException", "MAIL FROM domain is not verified."),
        "UNVERIFIED",
        "MailFromDomainNotVerifiedException",
        false,
    ],
    [
        "a suspended account",
        () => refused(400, "AccountSuspendedException", "Account suspended."),
        "SUSPENDED",
        "AccountSuspendedException",
        false,
    ],
    [
        "paused sending",
        () => refused(400, "SendingPausedException", "Sending paused."),
        "SUSPENDED",
        "SendingPausedException",
        false,
    ],
    [
        "a bad signature",
        () => refused(403, "InvalidSignatureException", "Signature mismatch."),
        "UNAUTHORIZED",
        "InvalidSignatureException",
        false,
    ],
    [
        "an unknown configuration set",
        () => refused(404, "NotFoundException", "Configuration set does not exist."),
        "INVALID_REQUEST",
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
        "INVALID_REQUEST",
        "BadRequestException",
        false,
    ],
    [
        "an answer without a MessageId",
        () => Response.json({}),
        "INVALID_RESPONSE",
        undefined,
        false,
    ],
])("map %s to its failure, keeping the AWS code", async (_, answer, code, awsCode, isRetryable) => {
    const client = new SesClient({
        region: "eu-central-1",
        credentials: async () => CREDENTIALS,
        fetch: fakeSes(answer).fetch,
    });

    const error = await client.send(ENVELOPE, MESSAGE).catch((caught: unknown) => caught);
    if (!(error instanceof SesError)) {
        throw new TypeError(`expected a SesError, got ${String(error)}`);
    }
    expect([error.code, error.awsCode, error.isRetryable]).toEqual([code, awsCode, isRetryable]);
});

test("map a failed connection to a retryable failure carrying its cause", async () => {
    const cause = new TypeError("fetch failed");
    const client = new SesClient({
        region: "eu-central-1",
        credentials: async () => CREDENTIALS,
        fetch: async () => {
            throw cause;
        },
    });

    const error = await client.send(ENVELOPE, MESSAGE).catch((caught: unknown) => caught);
    if (!(error instanceof SesError)) {
        throw new TypeError(`expected a SesError, got ${String(error)}`);
    }
    expect([error.code, error.isRetryable, error.cause]).toEqual(["CONNECTION", true, cause]);
});
