import { and, eq, isNotNull, type DatabaseConnection } from "@destack/db";
import {
    type Provider,
    type Provisioner,
    type ResourceRecord,
    type Rewrapper,
} from "@destack/resource";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { SPACE_ADDRESS } from "@destack/service/workload";
import { secret, secretVersion } from "../object/index.ts";
import { vault } from "../server/secret.ts";
import type { Keyring } from "@destack/host/keychain";
import { VaultKey } from "../encryption/index.ts";
import { VaultKind } from "../declare/kind.ts";
import { vaultKey } from "../stack/db.ts";

/** Keep vaults as keys in the space database wrapped under a host's keyring, their secrets served at the space's service. */
export class KeyringVaultHost implements Provisioner<typeof VaultKind> {
    /** The provider code of vaults, which their declarations' connectors name. */
    readonly provider = "vault";
    /** The space database keeping the vaults' keys and secrets. */
    readonly database: DatabaseConnection;
    /** The host's keyring wrapping the vaults' keys. */
    readonly keyring: Keyring;
    /** Where the keyring keeps its root keys, such as the host's region. */
    readonly location: string;
    /** Rewrap vault keys between hosts. */
    readonly rewrap: Rewrapper<typeof vaultKey, typeof vaultKey.$inferSelect>;

    /** Keep vaults in a space database under a keyring at a location. */
    constructor(database: DatabaseConnection, keyring: Keyring, location: string) {
        // keep the database and keyring, and rewrap keys with them
        this.database = database;
        this.keyring = keyring;
        this.location = location;
        this.rewrap = {
            table: vaultKey,
            wrap: async (row, recipient) => {
                // wrap the key for the target, which no other host unwraps
                const sealed = await VaultKey.transfer(
                    keyring,
                    location,
                    row.vaultId,
                    row,
                    recipient,
                );

                return { ...row, ...sealed };
            },
            unwrap: async (row, recipient) => {
                // wrap the key under this host's root keys
                const wrapped = await VaultKey.receive(
                    keyring,
                    location,
                    row.vaultId,
                    row,
                    recipient,
                );

                return { ...row, ...wrapped };
            },
        };
    }

    /** Keep a vault's key. */
    async provision(resource: ResourceRecord<typeof VaultKind>): Promise<{ reference: string }> {
        const target = { id: schema.identifier("vault").parse(resource.id), scope: resource.scope };
        await VaultKey.provision(this.database, this.keyring, this.location, target);

        return { reference: SPACE_ADDRESS };
    }

    /** Delete a vault's key, refusing a vault that still holds secret values. */
    async destroy(resource: ResourceRecord<typeof VaultKind>): Promise<void> {
        // refuse destroying a vault with secret values
        const vaultId = schema.identifier("vault").parse(resource.id);
        const [stored] = await this.database
            .select({ secretId: secretVersion.table.parentId })
            .from(secretVersion.table)
            .innerJoin(secret.table, eq(secret.table.id, secretVersion.table.parentId))
            .where(and(eq(secret.table.parentId, vaultId), isNotNull(secretVersion.table.envelope)))
            .limit(1);
        if (stored !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `vault has values of secret ${stored.secretId}: purge its secrets first`,
            });
        }

        // delete its key
        await VaultKey.discard(this.database, vaultId);
    }
}

/** A vault provider: provisioning one host's vaults and rewrapping their keys between hosts. */
export type VaultProvider = Provider<
    typeof VaultKind,
    typeof vault,
    never,
    typeof vaultKey,
    typeof vaultKey.$inferSelect
> & {
    readonly provision: Provisioner<typeof VaultKind>;
    readonly rewrap: Rewrapper<typeof vaultKey, typeof vaultKey.$inferSelect>;
};

/** Provide a host's vaults under its provider code, their secrets served by the space's service. */
export function vaultProvider(host: KeyringVaultHost): VaultProvider {
    return {
        kind: VaultKind,
        code: host.provider,
        object: vault,
        provision: host,
        rewrap: host.rewrap,
    };
}
