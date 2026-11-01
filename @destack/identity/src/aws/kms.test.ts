import { expect, refusal, test } from "@destack/test";
import { schema } from "@destack/schema";
import { testSimulation } from "@destack/test/world";
import { TEST_KMS_DENIED_KEY_ID, TEST_KMS_KEY_ID, TestKms } from "../test/index.ts";

/** The simulated node KMS clients run on. */
const node = testSimulation().environment("kms");

/** A sealed keyset's versions, as the tests read them. */
const Sealed = schema.looseObject({ keys: schema.record(schema.string(), schema.string()) });

/** The label the tests derive a secret under. */
const LABEL = { label: "@destack/identity/test", context: "" };

test("seal a root key inside KMS, unseal the same secrets on every opening, and rotate beside the first version", async () => {
    // seal a first version, then a second beside it
    const kms = new TestKms(node);
    const first = await kms.client().seal(TEST_KMS_KEY_ID, "machine eu-1");
    const second = await kms.client().seal(TEST_KMS_KEY_ID, "machine eu-1", first);

    // unseal both openings of the first keyset to the same secret, and the rotated one to both versions
    const opened = await kms.client().unseal(first, "machine eu-1", node);
    const reopened = await kms.client().unseal(first, "machine eu-1", node);
    const rotated = await kms.client().unseal(second, "machine eu-1", node);
    const derived = await rotated.deriveEach(LABEL);
    expect({
        stored: Object.keys(Sealed.parse(JSON.parse(second)).keys),
        same: (await opened.derive(LABEL)).toHex() === (await reopened.derive(LABEL)).toHex(),
        active: rotated.active,
        versions: derived.map(({ version }) => version),
        kept: derived[1]?.secret.toHex() === (await opened.derive(LABEL)).toHex(),
        operations: kms.operations,
    }).toEqual({
        stored: ["1", "2"],
        same: true,
        active: 2,
        versions: [2, 1],
        kept: true,
        operations: [
            "GenerateDataKeyWithoutPlaintext",
            "GenerateDataKeyWithoutPlaintext",
            "Decrypt",
            "Decrypt",
            "Decrypt",
            "Decrypt",
        ],
    });
});

test("refuse unsealing for another owner, under a denied key, and continuing a keyset under another key", async () => {
    // seal for one owner and a keyset naming the denied key
    const kms = new TestKms(node);
    const sealed = await kms.client().seal(TEST_KMS_KEY_ID, "machine eu-1");
    const denied = JSON.stringify({
        ...Sealed.parse(JSON.parse(sealed)),
        keyId: TEST_KMS_DENIED_KEY_ID,
    });

    // refuse each with KMS's own reason
    expect({
        owner: await refusal(kms.client().unseal(sealed, "machine us-1", node)),
        denied: await refusal(kms.client().unseal(denied, "machine eu-1", node)),
        other: await refusal(kms.client().seal(TEST_KMS_DENIED_KEY_ID, "machine eu-1", sealed)),
    }).toEqual({
        owner: [
            "INTERNAL_SERVER_ERROR",
            "DECRYPTION_FAILED",
            "KMS refused Decrypt with HTTP 400 InvalidCiphertextException: The ciphertext is invalid.",
        ],
        denied: [
            "INTERNAL_SERVER_ERROR",
            "KMS_REFUSED",
            "KMS refused Decrypt with HTTP 400 AccessDeniedException: User is not authorized to use the key.",
        ],
        other: [
            "INTERNAL_SERVER_ERROR",
            "ROOT_KEY_INVALID",
            `the keyset is sealed under ${TEST_KMS_KEY_ID}, not ${TEST_KMS_DENIED_KEY_ID}`,
        ],
    });
});
