import { expect, test } from "@destack/test";
import { Digest } from "../src/object/digest.ts";

/** Hash text with SHA-256 into the encodings of the published test vectors. */
test("hash text into RFC 7636 S256 challenges and FIPS 180-2 hex digests", async () => {
    // derive the challenge of RFC 7636 appendix B, and the digest of FIPS 180-2's "abc"
    expect([
        await Digest.base64url("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        await Digest.hex("abc"),
    ]).toEqual([
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ]);
});
