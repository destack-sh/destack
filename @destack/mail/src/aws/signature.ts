import { Sha256 } from "@aws-crypto/sha256-browser";
import { SignatureV4 } from "@smithy/signature-v4";

/** An AWS identity's credentials: an access key, its secret, and a session token for temporary ones. */
export interface AwsCredentials {
    /** The access key id. */
    readonly accessKeyId: string;
    /** The secret access key. */
    readonly secretAccessKey: string;
    /** The session token of temporary credentials. */
    readonly sessionToken?: string;
}

/** Open the upstream Signature Version 4 signer of an AWS service in a region over WebCrypto, asking the credentials on each signature. */
export function awsSigner(options: {
    readonly service: string;
    readonly region: string;
    readonly credentials: () => Promise<AwsCredentials>;
}): SignatureV4 {
    return new SignatureV4({
        service: options.service,
        region: options.region,
        credentials: options.credentials,
        sha256: Sha256,
        applyChecksum: false,
    });
}
