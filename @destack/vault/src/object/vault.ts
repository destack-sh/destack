import { relation, through } from "@destack/access";
import { account, organization, space } from "@destack/account/object";
import { defineObject } from "@destack/object";
import { schema } from "@destack/schema";
import { VaultKind } from "../declare/kind.ts";
import { SecretReading, SecretValue } from "@destack/resource";
import { SecretDisclosure, VaultSelection } from "../secret/secret.ts";
import { provisioned } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A vault of secrets in a space, account or organization: a resource of the vault kind. */
export const vault = defineObject(
    {
        name: "vault",
        plural: "vaults",
        title: t`${plural(argument("count"), { one: "Vault", other: "Vaults" })}`,
        scope: [space, account, organization],
        fields: {},
        access: {
            // let its consumers read and write its secrets by name
            permissions: {
                consume: relation("consumer"),
                // let an account's home declare the account's vaults
                declare: through("scope", "provision"),
            },
            reserved: ["consume"],
            suspended: ["read"],
        },
        traits: [provisioned({ kind: VaultKind })],
    },
    (method) => ({
        /** Read a version of the vault's secret of a name, as the secret's readers. */
        read: method.query({
            title: t`Read`,
            permission: "read",
            input: VaultSelection,
            output: SecretReading,
            audit: { details: SecretDisclosure },
        }),
        /** Put a value as the one live version of a secret of the vault, as its consumer. */
        put: method.mutation({
            title: t`Put`,
            permission: "write",
            visibility: "installation",
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
            title: t`Remove`,
            permission: "write",
            visibility: "installation",
            input: schema.object({
                /** The secret. */
                secret: schema.identifier("secret"),
            }),
        }),
    }),
);
