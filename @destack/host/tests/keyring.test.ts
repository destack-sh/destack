import { expect, test } from "@destack/test";
import { HostError } from "../src/error/index.ts";
import { LocalKeyring, MemoryKeychain } from "../src/keychain/index.ts";

test("open root keys a keychain keeps, refuse another context, rotate them, and retire the old key once rewrapped, refusing the active and unknown keys", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = new TextEncoder().encode("context-1");
    const dataKey = crypto.getRandomValues(new Uint8Array(32));

    // generate the first key once, and open it again
    const first = await LocalKeyring.open(keychain, name);
    expect((await LocalKeyring.open(keychain, name)).active).toBe(first.active);
    const wrapped = await first.wrap(dataKey, context);

    // refuse unwrapping under another context
    await expect(first.unwrap(wrapped, new TextEncoder().encode("context-2"))).rejects.toEqual(
        new HostError("DECRYPTION_FAILED", "data key authentication failed"),
    );

    // rotate to a new active key that still unwraps the old key's data keys
    const rotated = await LocalKeyring.rotate(keychain, name);
    const rewrapped = await rotated.wrap(await rotated.unwrap(wrapped, context), context);
    expect([rotated.active === first.active, rewrapped.keyId]).toEqual([false, rotated.active]);

    // refuse retiring the active key or an unknown one, then retire the old one
    await expect(LocalKeyring.retire(keychain, name, rotated.active)).rejects.toEqual(
        new HostError("INVALID_KEY", "cannot retire the active root key"),
    );
    await expect(LocalKeyring.retire(keychain, name, "unknown")).rejects.toEqual(
        new HostError("INVALID_KEY", `no root key unknown is kept as ${name}`),
    );
    const retired = await LocalKeyring.retire(keychain, name, first.active);
    await expect(retired.unwrap(wrapped, context)).rejects.toEqual(
        new HostError("KEY_UNAVAILABLE", "required root key is unavailable"),
    );
    expect([...(await retired.unwrap(rewrapped, context))]).toEqual([...dataKey]);
});

test("open one keyset from concurrent opens of an empty keychain, and keep both of two concurrent rotations", async () => {
    const keychain = new MemoryKeychain();
    const name = "keys/host-1";
    const context = new TextEncoder().encode("context-1");
    const dataKey = crypto.getRandomValues(new Uint8Array(32));

    // adopt the winner's keyset in the opening that lost the race, so each unwraps the other's keys
    const [first, second] = await Promise.all([
        LocalKeyring.open(keychain, name),
        LocalKeyring.open(keychain, name),
    ]);
    const wrapped = await first.wrap(dataKey, context);
    expect([second.active, [...(await second.unwrap(wrapped, context))]]).toEqual([
        first.active,
        [...dataKey],
    ]);

    // stack the rotation that lost the race over the winner's key, keeping every key
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
