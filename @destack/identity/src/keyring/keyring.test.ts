import { schema } from "@destack/schema";
import { expect, test, refusal } from "@destack/test";
import { MemoryKeychain } from "../keychain/index.ts";
import { LocalKeyring } from "./index.ts";

test("open root keys a keychain stores, refuse another context, rotate them, and retire the old key once reencrypted, refusing the active and unknown keys", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = { label: "@destack/identity/test", context: "context-1" };
    const plaintext = crypto.getRandomValues(new Uint8Array(32));

    // generate the first key once
    const first = await LocalKeyring.open(keychain, name);
    expect((await LocalKeyring.open(keychain, name)).active).toBe(first.active);
    const encrypted = await first.encrypt(plaintext, context);

    // refuse decrypting under another context
    expect(
        await refusal(
            first.decrypt(encrypted, { label: "@destack/identity/test", context: "context-2" }),
        ),
    ).toEqual(["INTERNAL_SERVER_ERROR", "DECRYPTION_FAILED", "ciphertext authentication failed"]);

    // rotate to a new active key that still decrypts the old key's ciphertexts
    const rotated = await LocalKeyring.rotate(keychain, name);
    const reencrypted = await rotated.encrypt(await rotated.decrypt(encrypted, context), context);
    expect([rotated.active === first.active, LocalKeyring.version(reencrypted)]).toEqual([
        false,
        rotated.active,
    ]);

    // retire only the old key
    expect(await refusal(LocalKeyring.retire(keychain, name, rotated.active))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "INVALID_ROOT_KEY",
        "cannot retire the active root key",
    ]);
    expect(await refusal(LocalKeyring.retire(keychain, name, 99))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "INVALID_ROOT_KEY",
        `root key 99 not found in ${name}`,
    ]);
    const retired = await LocalKeyring.retire(keychain, name, first.active);
    expect(await refusal(retired.decrypt(encrypted, context))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "KEY_UNAVAILABLE",
        "required root key is unavailable",
    ]);
    expect([...(await retired.decrypt(reencrypted, context))]).toEqual([...plaintext]);
});

test("open one keyset from concurrent opens of an empty keychain, and store both of two concurrent rotations", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = { label: "@destack/identity/test", context: "context-1" };
    const plaintext = crypto.getRandomValues(new Uint8Array(32));

    // adopt the winner's keyset in the opening that lost the race
    const [first, second] = await Promise.all([
        LocalKeyring.open(keychain, name),
        LocalKeyring.open(keychain, name),
    ]);
    const encrypted = await first.encrypt(plaintext, context);
    expect([second.active, [...(await second.decrypt(encrypted, context))]]).toEqual([
        first.active,
        [...plaintext],
    ]);

    // stack the rotation that lost the race over the winner's key
    const [left, right] = await Promise.all([
        LocalKeyring.rotate(keychain, name),
        LocalKeyring.rotate(keychain, name),
    ]);
    const reopened = await LocalKeyring.open(keychain, name);
    await LocalKeyring.retire(keychain, name, left.active);
    const retired = await LocalKeyring.retire(keychain, name, first.active);
    expect([reopened.active, retired.active, keychain.secrets.size]).toEqual([
        right.active,
        right.active,
        1,
    ]);
});

test("derive the same secrets and private key from a keyset's text on every opening, apart by label and by keyset", async () => {
    // open one keyset twice beside another
    const text = LocalKeyring.generate();
    const [first, second, other] = await Promise.all([
        LocalKeyring.read(text),
        LocalKeyring.read(text),
        LocalKeyring.read(LocalKeyring.generate()),
    ]);

    // derive alike only from the same keyset and label
    const machine = await first.derivePrivateKey({ label: "machine", context: "" });
    const signer = await crypto.subtle.importKey(
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
        machine.d === (await second.derivePrivateKey({ label: "machine", context: "" })).d,
        machine.d === (await first.derivePrivateKey({ label: "relay", context: "" })).d,
        signer.type,
    ]).toEqual([true, false, false, true, false, "private"]);
});

test("derive one keyset per label with every root key version, the same on every opening and apart by label", async () => {
    // rotate a keyset to two versions, and derive two labels' keysets from it twice
    const keychain = new MemoryKeychain();
    await LocalKeyring.open(keychain, "master");
    const master = await LocalKeyring.rotate(keychain, "master");
    const reopened = await LocalKeyring.open(keychain, "master");
    const universe = await master.deriveKeyset({ label: "universe", context: "" });
    const eu = await master.deriveKeyset({ label: "eu", context: "" });

    // keep the versions and the active one, derive alike only under one label, and open as a keyset
    const read = await LocalKeyring.read(universe);
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
