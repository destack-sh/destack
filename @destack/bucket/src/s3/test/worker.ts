import { EMPTY_HASH } from "@destack/identity/aws";
import { S3Signature } from "../signature.ts";

/** The example credentials of the AWS Signature Version 4 documentation. */
const CREDENTIALS = {
    accessKeyId: "AKIAIOSFODNN7EXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
};

/** The signing time of the documentation's examples, 20130524T000000Z. */
const NOW = Date.parse("2013-05-24T00:00:00Z");

/** The Worker signing, presigning and verifying the documentation's examples. */
export default {
    /** Answer the signed authorization, the presigned URL and the access keys both verify as. */
    async fetch(): Promise<Response> {
        // sign the ranged GET and presign the plain GET
        const signer = new S3Signature({ region: "us-east-1" });
        const signed = await signer.sign(
            new Request("https://examplebucket.s3.amazonaws.com/test.txt", {
                headers: { range: "bytes=0-9", "x-amz-content-sha256": EMPTY_HASH },
            }),
            CREDENTIALS,
            NOW,
        );
        const presigned = await signer.presign(
            new Request("https://examplebucket.s3.amazonaws.com/test.txt"),
            CREDENTIALS,
            86400,
            NOW,
        );

        // verify both
        const lookup = async () => CREDENTIALS;
        const verified = await Promise.all(
            [signed, presigned].map(async (request) => {
                const authorization = await signer.authenticate(request, lookup, NOW);

                return authorization.accessKeyId;
            }),
        );

        return Response.json({
            authorization: signed.headers.get("authorization"),
            url: presigned.url,
            verified,
        });
    },
};
