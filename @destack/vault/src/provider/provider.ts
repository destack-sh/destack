import { and, eq, isNotNull, type DatabaseConnection } from "@destack/db";
import { type Provider, type Provision, type Rewrap } from "@destack/resource";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { secret, secretVersion } from "../object/index.ts";
import { vault } from "../server/secret.ts";
import { type Keyring, VaultKey } from "../encryption/index.ts";
import { VaultKind } from "../declare/vault.ts";
import { vaultKey } from "../stack/db.ts";

/** Provide vaults as keys in the space database. */
export function vaultProvider(
    database: DatabaseConnection,
    keyring: Keyring,
    location: string,
): Provider<typeof VaultKind, typeof vault> &
    Provision<typeof VaultKind> &
    Rewrap<typeof vaultKey, typeof vaultKey.$inferSelect> {
    return {
        kind: VaultKind,
        code: "vault",
        object: vault,
        table: vaultKey,
        wrap: async (row, recipient) => {
            // wrap the key for the target, which no other host unwraps
            const sealed = await VaultKey.transfer(keyring, location, row.vaultId, row, recipient);

            return { ...row, ...sealed };
        },
        unwrap: async (row, recipient) => {
            // wrap the key under this host's root keys
            const wrapped = await VaultKey.receive(keyring, location, row.vaultId, row, recipient);

            return { ...row, ...wrapped };
        },
        provision: async (resource) => {
            // keep the vault's key
            const vault = { id: identifier("vault").parse(resource.id), scope: resource.scope };
            await VaultKey.provision(database, keyring, location, vault);

            return { reference: resource.id };
        },
        destroy: async (resource) => {
            // refuse destroying a vault with secret values
            const [stored] = await database
                .select({ secretId: secretVersion.table.parentId })
                .from(secretVersion.table)
                .innerJoin(secret.table, eq(secret.table.id, secretVersion.table.parentId))
                .where(
                    and(
                        eq(secret.table.parentId, identifier("vault").parse(resource.id)),
                        isNotNull(secretVersion.table.envelope),
                    ),
                )
                .limit(1);
            if (stored !== undefined) {
                throw new ServiceError("CONFLICT", {
                    message: `vault has values of secret ${stored.secretId}: purge its secrets first`,
                });
            }

            // delete its key
            await VaultKey.discard(database, identifier("vault").parse(resource.id));
        },
    };
}
