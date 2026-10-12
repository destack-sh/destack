import { through } from "@destack/access";
import { Ciphertext } from "@destack/identity";
import { defineObject, field } from "@destack/object";
import { account, organisation, space } from "@destack/account/object";
import { vault } from "./vault.ts";

/** One numbered version of a vault's key. */
export const vaultKey = defineObject({
    name: "vault-key",
    plural: "vaultKeys",
    scope: [space, account, organisation],
    nested: { in: vault, delete: "cascade", receive: "declare" },
    versioned: true,
    audited: { reads: true },
    fields: {
        /** The key's material, encrypted under the serving machine's keyring, in no output. */
        ciphertext: field.string(Ciphertext).sensitive(),
    },
    // let the vault's readers list its keys and its declarers rotate them
    permissions: { read: through("parent", "read"), rotate: through("parent", "declare") },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    /** Generate the vault's next key version, which encrypts every new value. */
    rotate: method.create("rotate", { fields: [] }),
}));
