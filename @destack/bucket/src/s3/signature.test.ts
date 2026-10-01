import { expect, test } from "@destack/test";
import { Payload } from "./payload.ts";
import { SignatureV4 } from "./signature.ts";

/** The example credentials of the AWS Signature Version 4 documentation. */
const CREDENTIALS = {
    accessKeyId: "AKIAIOSFODNN7EXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
};
/** The signing time of the documentation's examples, 20130524T000000Z. */
const NOW = Date.parse("2013-05-24T00:00:00Z");
/** The signer of the documentation's examples. */
const SIGNER = new SignatureV4({ region: "us-east-1" });

test("presign the documented GET example", async () => {
    // sign the example URL for 86400 seconds
    const presigned = await SIGNER.presign(
        new Request("https://examplebucket.s3.amazonaws.com/test.txt"),
        CREDENTIALS,
        86400,
        NOW,
    );
    const url = new URL(presigned.url);
    expect(url.href).toBe(
        "https://examplebucket.s3.amazonaws.com/test.txt?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host&X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404",
    );

    // accept it until it expires
    const lookup = async () => CREDENTIALS;
    expect(
        (await SIGNER.authenticate(new Request(url.href), lookup, NOW + 86400 * 1000)).accessKeyId,
    ).toBe(CREDENTIALS.accessKeyId);
    await expect(
        SIGNER.authenticate(new Request(url.href), lookup, NOW + 86401 * 1000),
    ).rejects.toMatchObject({
        code: "AccessDenied",
        message: "request has expired",
    });
});

test("sign the documented ranged GET example", async () => {
    // sign the example request with its range and empty payload hash
    const request = await SIGNER.sign(
        new Request("https://examplebucket.s3.amazonaws.com/test.txt", {
            headers: {
                range: "bytes=0-9",
                "x-amz-content-sha256":
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            },
        }),
        CREDENTIALS,
        NOW,
    );
    expect(request.headers.get("authorization")).toBe(
        "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41",
    );

    // accept it within the clock skew and refuse it after
    const lookup = async () => CREDENTIALS;
    expect((await SIGNER.authenticate(request, lookup, NOW + 15 * 60 * 1000)).accessKeyId).toBe(
        CREDENTIALS.accessKeyId,
    );
    await expect(
        SIGNER.authenticate(request, lookup, NOW + 15 * 60 * 1000 + 1),
    ).rejects.toMatchObject({
        code: "RequestTimeTooSkewed",
    });
});

test("decode the documented signed aws-chunked upload", async () => {
    // build the example body of 65536 and 1024 bytes and its chunk signatures
    const chunk = (size: number, signature: string) =>
        `${size.toString(16)};chunk-signature=${signature}\r\n${"a".repeat(size)}\r\n`;
    const body = [
        chunk(65536, "ad80c730a21e5b8d04586a2213dd63b9a0e99e0e2307b0ade35a65485a288648"),
        chunk(1024, "0055627c9e194cb4542bae2aa5492e3c1575bbb81b612b7d234b86a503ef5497"),
        chunk(0, "b6c6ea8a5354eaf15b3cb7646744f4275b71ea724fed81ceb9323e279d449df9"),
    ].join("");
    const request = (content: string) =>
        new Request("https://s3.amazonaws.com/examplebucket/chunkObject.txt", {
            method: "PUT",
            headers: {
                host: "s3.amazonaws.com",
                "x-amz-date": "20130524T000000Z",
                "x-amz-storage-class": "REDUCED_REDUNDANCY",
                authorization:
                    "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,SignedHeaders=content-encoding;content-length;host;x-amz-content-sha256;x-amz-date;x-amz-decoded-content-length;x-amz-storage-class,Signature=4f232c4386841ef735655705268965c44a0e4690baa4adea153f7db9fa80a0a9",
                "x-amz-content-sha256": "STREAMING-AWS4-HMAC-SHA256-PAYLOAD",
                "content-encoding": "aws-chunked",
                "x-amz-decoded-content-length": "66560",
                "content-length": "66824",
            },
            body: content,
        });

    // verify the seed signature, then every chunk signature while decoding
    const signed = request(body);
    const authorization = await SIGNER.authenticate(signed, async () => CREDENTIALS, NOW);
    const decoded = await new Response(Payload.open(signed, signed.headers, authorization)).text();
    expect(decoded).toBe("a".repeat(66560));

    // refuse a changed chunk
    const changed = request(body.replace("aaaa\r\n400", "aaab\r\n400"));
    const stream = Payload.open(changed, changed.headers, authorization);
    await expect(new Response(stream).text()).rejects.toMatchObject({
        code: "SignatureDoesNotMatch",
        message: "an aws-chunked chunk signature does not match",
    });
});
