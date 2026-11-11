import { expect, refusal, test } from "@destack/test";
import { LocalKeyring } from "../keyring/index.ts";
import { Envelope } from "./index.ts";
import { testSimulation } from "@destack/test/world";
import { TEST_KMS_CREDENTIALS, TEST_KMS_KEY_ID, TEST_KMS_REGION, TestKms } from "../test/index.ts";

/** The simulated node KMS clients run on. */
const node = testSimulation().environment("kms");

/** The label the tests derive a secret under. */
const LABEL = { label: "@destack/identity/test", context: "" };

/** The owner every secret is sealed for. */
const OWNER = "machine eu-central";

/** Open a envelope whose KMS calls a test KMS answers. */
function awsSeal(kms: TestKms, owner: string): Envelope {
    return new Envelope(
        { kind: "aws-kms", region: TEST_KMS_REGION, keyId: TEST_KMS_KEY_ID },
        owner,
        TEST_KMS_CREDENTIALS,
        {
            ...node,
            network: kms.network(node.network),
        },
    );
}

test("open a keyring and a secret through a KMS envelope for their owner, and the same in the clear through a local envelope", async () => {
    // seal a keyset and a secret for the owner
    const kms = new TestKms(node);
    const keyset = await kms.client().seal(TEST_KMS_KEY_ID, OWNER);
    const secret = await kms
        .client()
        .encrypt(TEST_KMS_KEY_ID, new TextEncoder().encode("key"), OWNER);
    const plain = LocalKeyring.generate(node);

    // open both through the KMS envelope, and the plain keyset and secret through a local one
    const sealed = awsSeal(kms, OWNER);
    const local = new Envelope({ kind: "local" }, OWNER, undefined, node);
    const unsealed = await sealed.keyring(keyset, node);
    const reopened = await kms.client().unseal(keyset, OWNER, node);
    expect({
        keyring: (await unsealed.derive(LABEL)).toHex() === (await reopened.derive(LABEL)).toHex(),
        secret: await sealed.open(secret),
        local: [
            (await (await local.keyring(plain, node)).derive(LABEL)).toHex() ===
                (await (await LocalKeyring.read(plain, node)).derive(LABEL)).toHex(),
            await local.open("key"),
        ],
    }).toEqual({ keyring: true, secret: "key", local: [true, "key"] });
});

test("refuse a secret sealed for another owner, and a KMS envelope without credentials", async () => {
    const kms = new TestKms(node);
    const secret = await kms
        .client()
        .encrypt(TEST_KMS_KEY_ID, new TextEncoder().encode("key"), OWNER);
    expect({
        owner: await refusal(awsSeal(kms, "machine us-east").open(secret)),
        credentials: await refusal(
            () =>
                new Envelope(
                    { kind: "aws-kms", region: TEST_KMS_REGION, keyId: TEST_KMS_KEY_ID },
                    OWNER,
                    undefined,
                    node,
                ),
        ),
    }).toEqual({
        owner: [
            "INTERNAL_SERVER_ERROR",
            "DECRYPTION_BROKEN",
            "KMS refused Decrypt with HTTP 400 InvalidCiphertextException: The ciphertext is invalid.",
        ],
        credentials: [
            "INTERNAL_SERVER_ERROR",
            "KMS_BROKEN",
            "the aws-kms envelope needs KMS credentials",
        ],
    });
});
