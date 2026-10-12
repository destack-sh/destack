import { Caller } from "@destack/access";
import { type DatabaseConnection, type Transaction } from "@destack/db";
import type { Keyring } from "@destack/identity";
import type { ObjectExtension, ObjectStore } from "@destack/object/server";
import { type Duration, type Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import type { ObjectReference } from "@destack/sync";
import { VaultKeystore } from "../key/index.ts";
import { vault } from "../object/index.ts";
import { vaultProvider } from "../provider/index.ts";
import { vaultService } from "../service/index.ts";
import { serveKeys, serveSecrets, serveVaults } from "./secret.ts";

/** The name of the vault every account and organisation keeps its services' credentials in. */
export const CREDENTIALS_VAULT = "credentials";

/** What a machine serves its spaces' vaults with. */
export interface VaultOptions {
    /** The space's database, keeping the vaults' keys beside them. */
    readonly database: DatabaseConnection;
    /** The keyring encrypting the vaults' keys. */
    readonly keyring: Keyring;
    /** The machine the vaults' keys belong to, which their encryption context includes. */
    readonly machine: Identifier<"machine">;
    /** How long deleted secrets stay restorable, 30 days by default. */
    readonly recovery?: Duration;
}

/** The vault service of a space, keeping each new scope's credentials vault. */
export interface VaultImplementation extends ObjectExtension {
    /** Upsert a scope's `credentials` vault with its key as the store's principal. */
    keep(
        store: Pick<ObjectStore, "mutate" | "principal">,
        scope: string,
        transaction: Transaction,
        now: number,
    ): Promise<ObjectReference>;
}

/** Implement the vault service: serve a space's vaults, secrets and versions. */
export function implementVault(options: VaultOptions): VaultImplementation {
    // serve the vaults with their keys and secrets, and the provider keeping them
    const keystore = new VaultKeystore(options.database, options.keyring, options.machine);
    const { secret, version } = serveSecrets(keystore, options.recovery);

    return {
        service: vaultService,
        objects: { vault: serveVaults(keystore), vaultKey: serveKeys(keystore), secret, version },
        providers: [vaultProvider(keystore)],
        keep: async (store, scope, transaction, now) => {
            // answer the scope's existing credentials vault
            const [existing] = await transaction.rows(vault.table, {
                scope,
                name: CREDENTIALS_VAULT,
            });
            if (existing !== undefined) {
                return vault.reference(scope, existing.id);
            }

            // create the scope's credentials vault with its first key in the transaction
            const id = schema.identifier("vault").parse(vault.generateId());
            await store.mutate(
                Caller.of(store.principal, now),
                {
                    id: RequestId.create(now),
                    calls: [
                        vault.call("create", {
                            id,
                            name: CREDENTIALS_VAULT,
                            definitionPackageId: vault.package.id,
                            definitionVersion: vault.package.version,
                            definitionName: CREDENTIALS_VAULT,
                            spec: {},
                        }),
                    ],
                },
                { transaction, scope },
            );

            return vault.reference(scope, id);
        },
    };
}
