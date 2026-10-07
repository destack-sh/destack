import { expect, test } from "@destack/test";
import { AwsSigner, EMPTY_HASH, type SignableRequest } from "./index.ts";

/** The AWS SigV4 test suite's example credentials. */
const CREDENTIALS = {
    accessKeyId: "AKIDEXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
};

/** The suite's signer of its example service. */
const SUITE = new AwsSigner({ region: "us-east-1", service: "service", credentials: CREDENTIALS });

/** The suite's signing time. */
const DATE = new Date("2015-08-30T12:36:00Z");

/** The suite's session token, which post-sts-header-after signs. */
const SESSION_TOKEN =
    "AQoDYXdzEPT//////////wEXAMPLEtc764bNrC9SAPBSM22wDOk4x4HIZ8j4FZTwdQWLWsKWHGBuFqwAeMicRXmxfpSPfIeoIYRqTflfKD8YUuwthAx7mSEI/qkPpKPi/kMcGdQrmGdeehM4IC1NtBmUpp2wUE8phUZampKsburEDy0KPkyQDYwT7WZ0wq5VSXDvp75YU9HFvlRd8Tx6q6fE8YQcHNVXAkiY9q6d+xo0rKwT38xVqr7ZD0u0iPPkUL64lIZbqBAz+scqKmlzm8FDrypNC9Yjc8fPOLn9FX9KSYvKTr4rvx3iSIlTJabIQwj2ICCR/oLxBA==";

/** The suite's Authorization prefix. */
const CREDENTIAL =
    "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request";

/** Describe a suite request to the example host with an empty body. */
function suiteRequest(method: string, url: string, headers: Record<string, string> = {}) {
    const parsed = new URL(url);

    return {
        method,
        path: AwsSigner.path(parsed.pathname, "double"),
        query: [...parsed.searchParams],
        headers: { host: parsed.host, ...headers },
        payloadHash: EMPTY_HASH,
    } satisfies SignableRequest;
}

test("sign the suite's get-vanilla and get-vanilla-query-order-key-case requests", async () => {
    const vanilla = await SUITE.authorize(
        suiteRequest("GET", "https://example.amazonaws.com/"),
        DATE,
    );
    const ordered = await SUITE.authorize(
        suiteRequest("GET", "https://example.amazonaws.com/?Param2=value2&Param1=value1"),
        DATE,
    );

    expect([vanilla, ordered["authorization"]]).toEqual([
        {
            host: "example.amazonaws.com",
            "x-amz-date": "20150830T123600Z",
            authorization: `${CREDENTIAL}, SignedHeaders=host;x-amz-date, Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31`,
        },
        `${CREDENTIAL}, SignedHeaders=host;x-amz-date, Signature=b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500`,
    ]);
});

test("sign the suite's post-x-www-form-urlencoded request over its content type and body", async () => {
    const request = {
        ...suiteRequest("POST", "https://example.amazonaws.com/", {
            "Content-Type": "application/x-www-form-urlencoded",
        }),
        payloadHash: await AwsSigner.payloadHash("Param1=value1"),
    };
    const headers = await SUITE.authorize(request, DATE);

    expect(headers["authorization"]).toBe(
        `${CREDENTIAL}, SignedHeaders=content-type;host;x-amz-date, Signature=ff11897932ad3f4e8b18135d722051e5ac45fc38421b1da7b9d196a0fe09473a`,
    );
});

test("sign the suite's post-sts-header-after request with the session token as a signed header", async () => {
    const temporary = new AwsSigner({
        region: "us-east-1",
        service: "service",
        credentials: { ...CREDENTIALS, sessionToken: SESSION_TOKEN },
    });
    const headers = await temporary.authorize(
        suiteRequest("POST", "https://example.amazonaws.com/"),
        DATE,
    );

    expect([headers["x-amz-security-token"], headers["authorization"]]).toEqual([
        SESSION_TOKEN,
        `${CREDENTIAL}, SignedHeaders=host;x-amz-date;x-amz-security-token, Signature=85d96828115b5dc0cfc3bd16ad9e210dd772bbebba041836c64533a82be05ead`,
    ]);
});

test("encode each path segment again or once as S3 reads it, each query pair, and collapse header whitespace", async () => {
    const signed = await SUITE.sign(
        {
            ...suiteRequest("GET", "https://example.amazonaws.com/a b/ü?key=a b&key=A&tilde=~", {
                "X-Note": "  one   two  ",
            }),
        },
        DATE,
    );

    expect([
        signed.canonicalRequest,
        AwsSigner.path(new URL("https://example.com/a b/ü").pathname, "single"),
    ]).toEqual([
        [
            "GET",
            "/a%2520b/%25C3%25BC",
            "key=A&key=a%20b&tilde=~",
            "host:example.amazonaws.com",
            "x-note:one two",
            "",
            "host;x-note",
            EMPTY_HASH,
        ].join("\n"),
        "/a%20b/%C3%BC",
    ]);
});
