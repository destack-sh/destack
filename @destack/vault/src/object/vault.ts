import { none, through } from "@destack/access";
import { defineObject } from "@destack/object";
import { account, organisation, space } from "@destack/account/object";

import { VaultKind } from "../declare/kind.ts";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { SecretDisclosure, SecretReading, SecretValue, VaultSelection } from "../secret/secret.ts";

/** A space's vault of secrets: a resource of the vault kind. */
export const vault = defineObject({
    name: "vault",
    plural: "vaults",
    scope: [space, account, organisation],
    provisioned: { kind: VaultKind },
    fields: {},
    // let the account's servers declare its vaults
    permissions: { write: none(), declare: through("account", "serve") },
    administration: ["read"],
}).methods((method) => ({
    /** Put a value as the one live version of a secret of the vault, for a consumer. */
    put: method.mutation({
        permission: "write",
        isInternal: true,
        input: schema.object({
            /** The secret, created under this identifier on its first put. */
            secret: schema.identifier("secret"),
            /** The value, kept out of the journal. */
            value: schema.sensitive(SecretValue),
            /** The machine service reading the secret, bound to it as its consumer. */
            consumer: Subject,
        }),
    }),
    /** Remove a secret of the vault with its versions, releasing them from a consumer. */
    remove: method.mutation({
        permission: "write",
        isInternal: true,
        input: schema.object({
            /** The secret. */
            secret: schema.identifier("secret"),
            /** The machine service reading the secret. */
            consumer: Subject,
        }),
    }),
    /** Read a version of the vault's secret of a name, as the secret's openers. */
    read: method.query({
        permission: "read",
        input: VaultSelection,
        output: SecretReading,
        audit: { details: SecretDisclosure },
    }),
}));
