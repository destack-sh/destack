import { relation, through } from "@destack/access";
import { account, organization, space } from "@destack/account/object";
import { defineObject } from "@destack/object";
import { schema } from "@destack/schema";
import { VaultKind } from "../declare/kind.ts";
import { SecretDisclosure, SecretReading, SecretValue, VaultSelection } from "../secret/secret.ts";

/** A vault of secrets in a space, account or organization: a resource of the vault kind. */
export const vault = defineObject(
    {
        name: "vault",
        plural: "vaults",
        scope: [space, account, organization],
        provisioned: { kind: VaultKind },
        fields: {},
        // let its consumers read and write its secrets by name
        permissions: {
            consume: relation("consumer"),
            // let an account's home declare the account's vaults
            declare: through("scope", "provision"),
        },
        reserved: ["consume"],
        administration: ["read"],
    },
    (method) => ({
        /** Read a version of the vault's secret of a name, as the secret's readers. */
        read: method.query({
            permission: "read",
            input: VaultSelection,
            output: SecretReading,
            audit: { details: SecretDisclosure },
        }),
        /** Put a value as the one live version of a secret of the vault, as its consumer. */
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
            }),
        }),
        /** Remove a secret of the vault with its versions, releasing them, if it exists. */
        remove: method.mutation({
            permission: "write",
            isInternal: true,
            input: schema.object({
                /** The secret. */
                secret: schema.identifier("secret"),
            }),
        }),
    }),
);
