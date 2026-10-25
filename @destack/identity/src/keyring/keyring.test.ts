import { testSimulation } from "@destack/test/world";
import { schema } from "@destack/schema";
import { expect, test, refusal } from "@destack/test";
import { MemoryKeychain } from "../keychain/index.ts";
import { LocalKeyring } from "./index.ts";

test("open root keys a keychain stores, refuse another context, rotate them, and retire the old key once reencrypted, refusing the active and unknown keys", async () => {
    const world = testSimulation().environment("test");
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = { label: "@destack/identity/test", context: "context-1" };
    const plaintext = world.random.bytes(32);

    // generate the first key once
    const first = await LocalKeyring.open(keychain, name, world);
    expect((await LocalKeyring.open(keychain, name, world)).active).toBe(first.active);
    const encrypted = await first.encrypt(plaintext, context);

    // refuse decrypting under another context
    expect(
        await refusal(
            first.decrypt(encrypted, { label: "@destack/identity/test", context: "context-2" }),
        ),
    ).toEqual(["INTERNAL_SERVER_ERROR", "DECRYPTION_FAILED", "ciphertext authentication failed"]);

    // rotate to a new active key that still decrypts the old key's ciphertexts
    const rotated = await LocalKeyring.rotate(keychain, name, world);
    const reencrypted = await rotated.encrypt(await rotated.decrypt(encrypted, context), context);
    expect([rotated.active === first.active, LocalKeyring.version(reencrypted)]).toEqual([
        false,
        rotated.active,
    ]);

    // retire only the old key
    expect(await refusal(LocalKeyring.retire(keychain, name, rotated.active, world))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "ROOT_KEY_INVALID",
        "cannot retire the active root key",
    ]);
    expect(await refusal(LocalKeyring.retire(keychain, name, 99, world))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "ROOT_KEY_INVALID",
        `root key 99 not found in ${name}`,
    ]);
    const retired = await LocalKeyring.retire(keychain, name, first.active, world);
    expect(await refusal(retired.decrypt(encrypted, context))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "KEY_UNAVAILABLE",
        "required root key is unavailable",
    ]);
    expect([...(await retired.decrypt(reencrypted, context))]).toEqual([...plaintext]);
});

test("open one keyset from concurrent opens of an empty keychain, and store both of two concurrent rotations", async () => {
    const world = testSimulation().environment("test");
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = { label: "@destack/identity/test", context: "context-1" };
    const plaintext = world.random.bytes(32);

    // adopt the winner's keyset in the opening that lost the race
    const [first, second] = await Promise.all([
        LocalKeyring.open(keychain, name, world),
        LocalKeyring.open(keychain, name, world),
    ]);
    const encrypted = await first.encrypt(plaintext, context);
    expect([second.active, [...(await second.decrypt(encrypted, context))]]).toEqual([
        first.active,
        [...plaintext],
    ]);

    // stack the rotation that lost the race over the winner's key
    const [left, right] = await Promise.all([
        LocalKeyring.rotate(keychain, name, world),
        LocalKeyring.rotate(keychain, name, world),
    ]);
    const reopened = await LocalKeyring.open(keychain, name, world);
    await LocalKeyring.retire(keychain, name, left.active, world);
    const retired = await LocalKeyring.retire(keychain, name, first.active, world);
    expect([reopened.active, retired.active, keychain.secrets.size]).toEqual([
        right.active,
        right.active,
        1,
    ]);
});

test("derive the same secrets and private key from a keyset's text on every opening, apart by label and by keyset", async () => {
    // open one keyset twice beside another
    const world = testSimulation().environment("test");
    const text = LocalKeyring.generate(world);
    const [first, second, other] = await Promise.all([
        LocalKeyring.read(text, world),
        LocalKeyring.read(text, world),
        LocalKeyring.read(LocalKeyring.generate(world), world),
    ]);

    // derive alike only from the same keyset and label
    const machine = await first.derivePrivateKey({ label: "machine", context: "" });
    const signer = await world.crypto.importKey(
        "jwk",
        machine,
        { name: "ECDSA", namedCurve: "P-256" },
        false,
        ["sign"],
    );
    expect([
        (await first.derive({ label: "call", context: "" })).toHex() ===
            (await second.derive({ label: "call", context: "" })).toHex(),
        (await first.derive({ label: "call", context: "" })).toHex() ===
            (await first.derive({ label: "session", context: "" })).toHex(),
        (await first.derive({ label: "call", context: "" })).toHex() ===
            (await other.derive({ label: "call", context: "" })).toHex(),
        machine["d"] === (await second.derivePrivateKey({ label: "machine", context: "" }))["d"],
        machine["d"] === (await first.derivePrivateKey({ label: "relay", context: "" }))["d"],
        signer.type,
    ]).toEqual([true, false, false, true, false, "private"]);
});

test("derive one keyset per label with every root key version, the same on every opening and apart by label", async () => {
    // rotate a keyset to two versions, and derive two labels' keysets from it twice
    const world = testSimulation().environment("test");
    const keychain = new MemoryKeychain();
    await LocalKeyring.open(keychain, "master", world);
    const master = await LocalKeyring.rotate(keychain, "master", world);
    const reopened = await LocalKeyring.open(keychain, "master", world);
    const universe = await master.deriveKeyset({ label: "universe", context: "" });
    const eu = await master.deriveKeyset({ label: "eu", context: "" });

    // keep the versions and the active one, derive alike only under one label, and open as a keyset
    const read = await LocalKeyring.read(universe, world);
    const parsed = schema
        .object({ active: schema.number(), keys: schema.record(schema.string(), schema.string()) })
        .parse(JSON.parse(universe));
    expect({
        active: parsed.active,
        versions: Object.keys(parsed.keys).toSorted(),
        again: universe === (await reopened.deriveKeyset({ label: "universe", context: "" })),
        apart: universe === eu,
        derives:
            (await read.derive({ label: "call", context: "" })).toHex() ===
            (await master.derive({ label: "call", context: "" })).toHex(),
    }).toEqual({ active: 2, versions: ["1", "2"], again: true, apart: false, derives: false });
});
