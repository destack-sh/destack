import { Digest } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Credential } from "./credential.ts";

/** The length of every secret's prefix, such as dst_pat_. */
const PREFIX_LENGTH = 8;

test("create each kind of secret as its prefix before 32 random bytes in canonical base64url, beside its digest", async () => {
    // create a secret of each kind
    const credentials = [
        await Credential.create("personal-access-token"),
        await Credential.create("oauth-client"),
    ];

    // read each secret back as its kind, prefix and bytes, encoded again, and digest it again
    const read = await Promise.all(
        credentials.map(async ({ secret }) => {
            const bytes = Uint8Array.fromBase64(secret.slice(PREFIX_LENGTH), {
                alphabet: "base64url",
            });
            const encoded = bytes.toBase64({ alphabet: "base64url", omitPadding: true });

            return {
                kind: Credential.kind(secret),
                prefix: secret.slice(0, PREFIX_LENGTH),
                length: bytes.length,
                secret: `${secret.slice(0, PREFIX_LENGTH)}${encoded}`,
                digest: await Digest.of(secret),
            };
        }),
    );
    expect(read).toEqual(
        credentials.map(({ kind, secret, digest }, index) => ({
            kind,
            prefix: ["dst_pat_", "dst_ocs_"][index],
            length: 32,
            secret,
            digest,
        })),
    );
});
