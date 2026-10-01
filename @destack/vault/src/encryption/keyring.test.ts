import { MemoryKeychain } from "@destack/host/keychain";
import { expect, test } from "@destack/test";
import { VaultError } from "../error/index.ts";
import { LocalKeyring } from "./keyring.ts";

test("open root keys a keychain keeps, rotate them, and retire the old key once rewrapped, refusing the active and unknown keys", async () => {
    const keychain = new MemoryKeychain();
    const name = "vault/host-1";
    const context = new TextEncoder().encode("vault/secret-1/1");
    const dataKey = crypto.getRandomValues(new Uint8Array(32));

    // generate the first key once, and open it again
    const first = await LocalKeyring.open(keychain, name);
    expect((await LocalKeyring.open(keychain, name)).active).toBe(first.active);
    const wrapped = await first.wrap(dataKey, context);

    // rotate to a new active key that still unwraps the old key's data keys
    const rotated = await LocalKeyring.rotate(keychain, name);
    const rewrapped = await rotated.wrap(await rotated.unwrap(wrapped, context), context);
    expect([rotated.active === first.active, rewrapped.keyId]).toEqual([false, rotated.active]);

    // refuse retiring the active key or an unknown one, then retire the old one
    await expect(LocalKeyring.retire(keychain, name, rotated.active)).rejects.toEqual(
        new VaultError("INVALID_KEY", "the active root key cannot retire"),
    );
    await expect(LocalKeyring.retire(keychain, name, "unknown")).rejects.toEqual(
        new VaultError("INVALID_KEY", `no root key unknown is kept as ${name}`),
    );
    const retired = await LocalKeyring.retire(keychain, name, first.active);
    await expect(retired.unwrap(wrapped, context)).rejects.toEqual(
        new VaultError("KEY_UNAVAILABLE", "required root key is unavailable"),
    );
    expect([...(await retired.unwrap(rewrapped, context))]).toEqual([...dataKey]);
});
