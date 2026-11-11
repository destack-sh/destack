import { KeyPair } from "@destack/identity";
import { Envelope, type EnvelopeConfiguration } from "@destack/identity/aws";
import {
    TEST_KMS_CREDENTIALS,
    TEST_KMS_KEY_ID,
    TEST_KMS_REGION,
    TestKms,
} from "@destack/identity/test";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { Custody } from "./index.ts";

/** The label the test derives a secret under. */
const LABEL = { label: "@platform/release/test", context: "" };

/** The envelope of the test KMS's key. */
const SEAL: EnvelopeConfiguration = {
    kind: "aws-kms",
    region: TEST_KMS_REGION,
    keyId: TEST_KMS_KEY_ID,
};

/** The same key under another ARN, as a replaced residency key would be. */
const MOVED: EnvelopeConfiguration = { ...SEAL, keyId: `${TEST_KMS_KEY_ID}-moved` };

test("seal the keys a stage lacks inside KMS once, read them as each deploy does, refuse a moved key and a missing record, and open them as each Worker does", async () => {
    // seal the universe's and two machines' keys, then seal again with nothing missing
    const world = testSimulation().environment("operator");
    const directory = await world.files.makeTemporaryDirectory("destack-custody-");
    const file = `${directory}/test.json`;
    const kms = new TestKms(world);
    const environment = { ...world, network: kms.network(world.network) };
    const envelopes = (us: EnvelopeConfiguration) => ({
        universe: SEAL,
        machines: new Map([
            ["machine-eu", SEAL],
            ["machine-us", us],
        ]),
    });
    const first = await Custody.seal(file, envelopes(SEAL), TEST_KMS_CREDENTIALS, environment);
    const sealed = kms.operations.length;
    const again = await Custody.seal(file, envelopes(SEAL), TEST_KMS_CREDENTIALS, environment);
    const read = await Custody.read(file, envelopes(SEAL), environment);
    const keeping = kms.operations.length - sealed;

    // refuse a machine whose key moved, sealing and deploying alike, and a stage never sealed
    const moved =
        "the keys of machine machine-us are sealed under " +
        `${JSON.stringify(SEAL)}, not ${JSON.stringify(MOVED)}: rewrap them, never regenerate`;
    const missing = `${directory}/missing.json`;
    await expect(
        Custody.seal(file, envelopes(MOVED), TEST_KMS_CREDENTIALS, environment),
    ).rejects.toThrow(new Error(moved));
    await expect(Custody.read(file, envelopes(MOVED), environment)).rejects.toThrow(
        new Error(moved),
    );
    await expect(Custody.read(missing, envelopes(SEAL), environment)).rejects.toThrow(
        new Error(
            `the custody record '${missing}' is missing: the operator seals it with custody seal and commits it`,
        ),
    );
    await world.files.remove(directory, { recursive: true });

    // open the eu machine's root keys and key through its envelope, as its Worker does
    const keys = read.machine("machine-eu");
    const envelope = new Envelope(
        keys.envelope,
        Envelope.machine("machine-eu"),
        TEST_KMS_CREDENTIALS,
        environment,
    );
    const keyring = await envelope.keyring(keys.rootKey, environment);
    const opened = await KeyPair.read(await envelope.open(keys.key), environment.crypto);

    // seal three root keysets and two keys once, the second seal and the read keeping them as stored
    expect({
        operations: kms.operations.slice(0, sealed),
        keeping,
        again: JSON.stringify(again.record) === JSON.stringify(first.record),
        read: JSON.stringify(read.record) === JSON.stringify(first.record),
        publicKey: opened.publicKey,
        derives: (await keyring.derive(LABEL)).length,
    }).toEqual({
        operations: [
            "GenerateDataKeyWithoutPlaintext",
            "GenerateDataKeyWithoutPlaintext",
            "Encrypt",
            "GenerateDataKeyWithoutPlaintext",
            "Encrypt",
        ],
        keeping: 0,
        again: true,
        read: true,
        publicKey: keys.publicKey,
        derives: 32,
    });
});
