import { expect, test } from "@destack/test";
import { signRequest, type Signing } from "./index.ts";

/** The AWS SigV4 test suite's signer: its example credentials, region, service and time. */
const SUITE: Signing = {
    credentials: {
        accessKeyId: "AKIDEXAMPLE",
        secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
    },
    region: "us-east-1",
    service: "service",
    date: new Date("2015-08-30T12:36:00Z"),
};

/** The suite's session token, which post-sts-header-after signs. */
const SESSION_TOKEN =
    "AQoDYXdzEPT//////////wEXAMPLEtc764bNrC9SAPBSM22wDOk4x4HIZ8j4FZTwdQWLWsKWHGBuFqwAeMicRXmxfpSPfIeoIYRqTflfKD8YUuwthAx7mSEI/qkPpKPi/kMcGdQrmGdeehM4IC1NtBmUpp2wUE8phUZampKsburEDy0KPkyQDYwT7WZ0wq5VSXDvp75YU9HFvlRd8Tx6q6fE8YQcHNVXAkiY9q6d+xo0rKwT38xVqr7ZD0u0iPPkUL64lIZbqBAz+scqKmlzm8FDrypNC9Yjc8fPOLn9FX9KSYvKTr4rvx3iSIlTJabIQwj2ICCR/oLxBA==";

/** The suite's credential scope line. */
const SCOPE = "AWS4-HMAC-SHA256\n20150830T123600Z\n20150830/us-east-1/service/aws4_request";

/** The suite's Authorization prefix. */
const CREDENTIAL =
    "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request";

/** The digest of an empty body. */
const EMPTY = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

test("sign the suite's get-vanilla request to its canonical request, string to sign and signature", async () => {
    const signed = await signRequest(
        { method: "GET", url: new URL("https://example.amazonaws.com/"), headers: {}, body: "" },
        SUITE,
    );

    expect(signed).toEqual({
        canonicalRequest: [
            "GET",
            "/",
            "",
            "host:example.amazonaws.com",
            "x-amz-date:20150830T123600Z",
            "",
            "host;x-amz-date",
            EMPTY,
        ].join("\n"),
        stringToSign: `${SCOPE}\nbb579772317eb040ac9ed261061d46c1f17a8133879d6129b6e1c25292927e63`,
        authorization: `${CREDENTIAL}, SignedHeaders=host;x-amz-date, Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31`,
        headers: {
            host: "example.amazonaws.com",
            "x-amz-date": "20150830T123600Z",
            authorization: `${CREDENTIAL}, SignedHeaders=host;x-amz-date, Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31`,
        },
    });
});

test("sign the suite's post-x-www-form-urlencoded request over its content type and body", async () => {
    const signed = await signRequest(
        {
            method: "POST",
            url: new URL("https://example.amazonaws.com/"),
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: "Param1=value1",
        },
        SUITE,
    );

    expect([signed.canonicalRequest, signed.stringToSign, signed.authorization]).toEqual([
        [
            "POST",
            "/",
            "",
            "content-type:application/x-www-form-urlencoded",
            "host:example.amazonaws.com",
            "x-amz-date:20150830T123600Z",
            "",
            "content-type;host;x-amz-date",
            "9095672bbd1f56dfc5b65f3e153adc8731a4a654192329106275f4c7b24d0b6e",
        ].join("\n"),
        `${SCOPE}\n42a5e5bb34198acb3e84da4f085bb7927f2bc277ca766e6d19c73c2154021281`,
        `${CREDENTIAL}, SignedHeaders=content-type;host;x-amz-date, Signature=ff11897932ad3f4e8b18135d722051e5ac45fc38421b1da7b9d196a0fe09473a`,
    ]);
});

test("sign the suite's get-vanilla-query-order-key-case request with its query sorted by name", async () => {
    const signed = await signRequest(
        {
            method: "GET",
            url: new URL("https://example.amazonaws.com/?Param2=value2&Param1=value1"),
            headers: {},
            body: "",
        },
        SUITE,
    );

    expect([signed.canonicalRequest, signed.stringToSign, signed.authorization]).toEqual([
        [
            "GET",
            "/",
            "Param1=value1&Param2=value2",
            "host:example.amazonaws.com",
            "x-amz-date:20150830T123600Z",
            "",
            "host;x-amz-date",
            EMPTY,
        ].join("\n"),
        `${SCOPE}\n816cd5b414d056048ba4f7c5386d6e0533120fb1fcfa93762cf0fc39e2cf19e0`,
        `${CREDENTIAL}, SignedHeaders=host;x-amz-date, Signature=b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500`,
    ]);
});

test("sign the suite's post-sts-header-after request with the session token as a signed header", async () => {
    const signed = await signRequest(
        { method: "POST", url: new URL("https://example.amazonaws.com/"), headers: {}, body: "" },
        { ...SUITE, credentials: { ...SUITE.credentials, sessionToken: SESSION_TOKEN } },
    );

    expect([
        signed.headers["x-amz-security-token"],
        signed.stringToSign,
        signed.authorization,
    ]).toEqual([
        SESSION_TOKEN,
        `${SCOPE}\nc237e1b440d4c63c32ca95b5b99481081cb7b13c7e40434868e71567c1a882f6`,
        `${CREDENTIAL}, SignedHeaders=host;x-amz-date;x-amz-security-token, Signature=85d96828115b5dc0cfc3bd16ad9e210dd772bbebba041836c64533a82be05ead`,
    ]);
});

test("encode each path segment again and each query pair, and collapse header whitespace", async () => {
    const signed = await signRequest(
        {
            method: "GET",
            url: new URL("https://example.amazonaws.com/a b/ü?key=a b&key=A&tilde=~"),
            headers: { "X-Note": "  one   two  " },
            body: "",
        },
        SUITE,
    );

    expect(signed.canonicalRequest).toBe(
        [
            "GET",
            "/a%2520b/%25C3%25BC",
            "key=A&key=a%20b&tilde=~",
            "host:example.amazonaws.com",
            "x-amz-date:20150830T123600Z",
            "x-note:one two",
            "",
            "host;x-amz-date;x-note",
            EMPTY,
        ].join("\n"),
    );
});
