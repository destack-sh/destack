import { Simulation } from "@destack/environment/simulation";
import { expect, refusal, test } from "@destack/test";
import { Ciphertext } from "./ciphertext.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The context every vector was encrypted under. */
const LABEL = { label: "note", context: "space-1" };

/** The plaintext every vector carries. */
const PLAINTEXT = "the note's secret";

/** The AES-256 key the A256GCMKW vector wraps under: bytes 1 to 32. */
const WRAPPING_KEY = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";

/** A ciphertext jose 6.2.12's CompactEncrypt wrote under the key, with A256GCMKW. */
const WRAPPED =
    "eyJhbGciOiJBMjU2R0NNS1ciLCJraWQiOiJrZXktMSIsImVuYyI6IkEyNTZHQ00iLCJjdHgiOiJUbmxZVm9uZ3RuU2pPZlBkQmduRndjVnFSd0pLSzI5NFNxWllMTEo1bUtJIiwiaXYiOiIxSktfLUlCVUtPLW1IZS1nIiwidGFnIjoiRlFEd0Y2b1hCaHdqX3VDd2JfVjR6dyJ9.3j4gjiusff73A1i7xdrGUbP2fClU5w1WX4Yr6qta5E0.wB-mgyuBbHNQSyvf.-gkf5jiDQumTiCoI-hI0vcM.xTBCYDxG2Yr_eA69T5krVQ";

/** The P-256 recipient the ECDH-ES+A256KW vector agrees a key with. */
const RECIPIENT = {
    kty: "EC",
    crv: "P-256",
    ["d"]: "ECqhZeHToAvpwcdGE8Gmifa6ceRNBHkN7KRu_LOdyP8",
    x: "T4zxuxhpBtIQOPWXYjAEU-JAxbPq1WNYqrDYgFL9Ejk",
    y: "e00mQhlQbeIGrPeMglS6gWuWZM2G4VtM3Sm46eOxzRc",
};

/** A ciphertext jose 6.2.12's CompactEncrypt wrote to the recipient, with ECDH-ES+A256KW. */
const AGREED =
    "eyJhbGciOiJFQ0RILUVTK0EyNTZLVyIsImVuYyI6IkEyNTZHQ00iLCJjdHgiOiJUbmxZVm9uZ3RuU2pPZlBkQmduRndjVnFSd0pLSzI5NFNxWllMTEo1bUtJIiwiZXBrIjp7IngiOiJDdDBDaUZqbUM4b1JGUURVT2JZY1psUjR2LUE4S1h2bFN3VWRUeFl3ZUFnIiwiY3J2IjoiUC0yNTYiLCJrdHkiOiJFQyIsInkiOiJQSFc4R0xoejJNRklTVzhPQ000Y0l3MG1UYU8yMW5HdElCd3RwTVpkTUd3In19.On9mli3It7K2_a7eWUUqYx2MYoQw0IOKQnTjt46r8mFfffdQrn9gvg.9NRKYp2gBsRiUHbE.Vm9L1yxoZwsgIHuBfZ6t0vg.BL9mVZdc9_9UlqD4YnlECA";

test("decrypt the ciphertexts the jose library wrote, under a wrapped key and an agreed one, refusing another context", async () => {
    // import the wrapping key and the recipient's private key in a world
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("identity");
    const wrapping = await crypto.importKey(
        "raw",
        Uint8Array.fromHex(WRAPPING_KEY),
        "AES-GCM",
        false,
        ["encrypt", "decrypt"],
    );
    const recipient = await crypto.importKey(
        "jwk",
        RECIPIENT,
        { name: "ECDH", namedCurve: "P-256" },
        false,
        ["deriveBits"],
    );

    // decrypt each, then each under another context
    const decoder = new TextDecoder();
    const decrypted = [
        decoder.decode(await Ciphertext.decrypt(WRAPPED, wrapping, LABEL, crypto)),
        decoder.decode(await Ciphertext.decrypt(AGREED, recipient, LABEL, crypto)),
    ];
    const other = { label: "note", context: "space-2" };

    expect([
        decrypted,
        Ciphertext.keyId(WRAPPED),
        await refusal(Ciphertext.decrypt(WRAPPED, wrapping, other, crypto)),
        await refusal(Ciphertext.decrypt(AGREED, recipient, other, crypto)),
    ]).toEqual([
        [PLAINTEXT, PLAINTEXT],
        "key-1",
        ["INTERNAL_SERVER_ERROR", "DECRYPTION_FAILED", "ciphertext authentication failed"],
        ["INTERNAL_SERVER_ERROR", "DECRYPTION_FAILED", "ciphertext authentication failed"],
    ]);
});
