import { expect, test } from "@destack/test";
import { IdentityError } from "../error/error.ts";
import { MemoryKeychain } from "../keychain/index.ts";
import { LocalKeyring } from "./index.ts";

test("open root keys a keychain keeps, refuse another context, rotate them, and retire the old key once reencrypted, refusing the active and unknown keys", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = new TextEncoder().encode("context-1");
    const plaintext = crypto.getRandomValues(new Uint8Array(32));

    // generate the first key once
    const first = await LocalKeyring.open(keychain, name);
    expect((await LocalKeyring.open(keychain, name)).active).toBe(first.active);
    const encrypted = await first.encrypt(plaintext, context);

    // refuse decrypting under another context
    await expect(first.decrypt(encrypted, new TextEncoder().encode("context-2"))).rejects.toEqual(
        new IdentityError("DECRYPTION_FAILED", "ciphertext authentication failed"),
    );

    // rotate to a new active key that still decrypts the old key's ciphertexts
    const rotated = await LocalKeyring.rotate(keychain, name);
    const reencrypted = await rotated.encrypt(await rotated.decrypt(encrypted, context), context);
    expect([rotated.active === first.active, LocalKeyring.version(reencrypted)]).toEqual([
        false,
        rotated.active,
    ]);

    // retire only the old key
    await expect(LocalKeyring.retire(keychain, name, rotated.active)).rejects.toEqual(
        new IdentityError("INVALID_ROOT_KEY", "cannot retire the active root key"),
    );
    await expect(LocalKeyring.retire(keychain, name, 99)).rejects.toEqual(
        new IdentityError("INVALID_ROOT_KEY", `no root key 99 is kept as ${name}`),
    );
    const retired = await LocalKeyring.retire(keychain, name, first.active);
    await expect(retired.decrypt(encrypted, context)).rejects.toEqual(
        new IdentityError("KEY_UNAVAILABLE", "required root key is unavailable"),
    );
    expect([...(await retired.decrypt(reencrypted, context))]).toEqual([...plaintext]);
});

test("open one keyset from concurrent opens of an empty keychain, and keep both of two concurrent rotations", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = new TextEncoder().encode("context-1");
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
    const machine = await first.derivePrivateKey("machine");
    const signer = await crypto.subtle.importKey(
        "jwk",
        machine,
        { name: "ECDSA", namedCurve: "P-256" },
        false,
        ["sign"],
    );
    expect([
        (await first.derive("call")).toHex() === (await second.derive("call")).toHex(),
        (await first.derive("call")).toHex() === (await first.derive("session")).toHex(),
        (await first.derive("call")).toHex() === (await other.derive("call")).toHex(),
        machine.d === (await second.derivePrivateKey("machine")).d,
        machine.d === (await first.derivePrivateKey("relay")).d,
        signer.type,
    ]).toEqual([true, false, false, true, false, "private"]);
});
