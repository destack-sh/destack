import { relation, through } from "@destack/access";
import { defineObject } from "@destack/object";
import { account, organisation, space } from "@destack/account/object";

import { VaultKind } from "../declare/kind.ts";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { SecretDisclosure, SecretReading, SecretValue, VaultSelection } from "../secret/secret.ts";

/** A space's vault of secrets: a resource of the vault kind. */
export const vault = defineObject(
    {
        name: "vault",
        plural: "vaults",
        scope: [space, account, organisation],
        provisioned: { kind: VaultKind },
        fields: {},
        // let the account's servers declare its vaults, and its consumers read and write its secrets by name
        permissions: {
            declare: through("account", "serve"),
            consume: relation("consumer"),
        },
        reserved: ["consume"],
        administration: ["read"],
    },
    (method) => ({
        /** Read a version of the vault's secret of a name, as the secret's openers. */
        read: method.query({
            permission: "read",
            input: VaultSelection,
            output: SecretReading,
            audit: { details: SecretDisclosure },
        }),
        /** Put a value as the one live version of a secret of the vault, for a consumer. */
        put: method.mutation({
            permission: "write",
            isInternal: true,
            input: schema.object({
                /** The secret, created under this identifier on its first put. */
                secret: schema.identifier("secret"),
                /** The secret's name within the vault, given on its first put. */
                name: schema.string().min(1),
                /** The value, kept out of the journal. */
                value: schema.sensitive(SecretValue),
                /** The package reading the secret, bound to it as its consumer. */
                consumer: Subject,
            }),
        }),
        /** Remove a secret of the vault with its versions, releasing them, if it exists. */
        remove: method.mutation({
            permission: "write",
            isInternal: true,
            input: schema.object({
                /** The secret. */
                secret: schema.identifier("secret"),
                /** The package reading the secret. */
                consumer: Subject,
            }),
        }),
    }),
);
