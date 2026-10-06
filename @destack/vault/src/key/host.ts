import { and, type DatabaseConnection, eq, isNotNull } from "@destack/db";
import { type Ciphertext, type Keyring, LocalKeyring, type Recipient } from "@destack/identity";
import type { Provisioner, ResourceRecord, Rewrapper } from "@destack/resource";
import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { VAULT_PROVIDER, VaultKind } from "../declare/kind.ts";
import { secret, secretVersion } from "../object/index.ts";
import { vaultService } from "../service/index.ts";
import { vaultKey } from "../stack/db.ts";
import { VAULT_KEY_BYTES, VaultKey } from "./key.ts";

/** The protocol a vault key's encryption context names, beside its location and vault. */
const VAULT_KEY_PROTOCOL = "@destack/vault/key";

/** A vault's key as its table keeps it. */
type VaultKeyRow = typeof vaultKey.$inferSelect;

/** A host's vaults as keys encrypted under its keyring: provisioned with each vault, decrypted for its secrets' values, and sealed to the host a space moves to. */
export class KeyringVaultHost implements Provisioner<typeof VaultKind> {
    /** The provider code of vaults, which their declarations' connectors name. */
    readonly provider = VAULT_PROVIDER;
    /** The vault service's database keeping the vaults' keys and secrets. */
    readonly database: DatabaseConnection;
    /** The host's keyring encrypting the vaults' keys. */
    readonly keyring: Keyring;
    /** The host the vaults' keys belong to, which their encryption context names. */
    readonly location: string;
    /** The sealing of the keys to the host a space moves to. */
    readonly rewrap: Rewrapper<typeof vaultKey, VaultKeyRow>;

    /** Keep vaults' keys in the vault service's database under a host's keyring at a location. */
    constructor(database: DatabaseConnection, keyring: Keyring, location: string) {
        // keep the database, keyring and location, and seal keys with them
        this.database = database;
        this.keyring = keyring;
        this.location = location;
        this.rewrap = {
            table: vaultKey,
            seal: (row, recipient) => this.#seal(row, recipient),
            open: (row, recipient) => this.#open(row, recipient),
        };
    }

    /** Keep a new vault's key, answering the vault service's address, and keep the one a vault has. */
    async provision(
        resource: Pick<ResourceRecord<typeof VaultKind>, "id" | "scope">,
    ): Promise<{ reference: string }> {
        // generate a key for a vault without one
        const vaultId = vaultOf(resource);
        const [kept] = await this.database
            .select({ id: vaultKey.id })
            .from(vaultKey)
            .where(eq(vaultKey.vaultId, vaultId));
        if (kept === undefined) {
            const bytes = crypto.getRandomValues(new Uint8Array(VAULT_KEY_BYTES));
            try {
                await this.database.insert(vaultKey).values({
                    vaultId,
                    scope: schema.identifier("space").parse(resource.scope),
                    id: crypto.randomUUID(),
                    ciphertext: await this.keyring.encrypt(bytes, this.#context(vaultId)),
                });
            } finally {
                bytes.fill(0);
            }
        }

        return { reference: vaultService.package.name };
    }

    /** Delete a vault's key, shredding every value it encrypted, refusing a vault that still holds secret values. */
    async destroy(resource: ResourceRecord<typeof VaultKind>): Promise<void> {
        // refuse destroying a vault with secret values
        const vaultId = vaultOf(resource);
        const [stored] = await this.database
            .select({ secretId: secretVersion.table.parentId })
            .from(secretVersion.table)
            .innerJoin(secret.table, eq(secret.table.id, secretVersion.table.parentId))
            .where(
                and(eq(secret.table.parentId, vaultId), isNotNull(secretVersion.table.ciphertext)),
            )
            .limit(1);
        if (stored !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `vault has values of secret ${stored.secretId}: purge its secrets first`,
            });
        }

        // delete its key
        await this.database.delete(vaultKey).where(eq(vaultKey.vaultId, vaultId));
    }

    /** Decrypt a vault's key in a call's database, for its secrets' values, refusing a vault without one. */
    async key(database: DatabaseConnection, vaultId: Identifier<"vault">): Promise<VaultKey> {
        // read the vault's key
        const [kept] = await database.select().from(vaultKey).where(eq(vaultKey.vaultId, vaultId));
        if (kept === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `vault ${vaultId} has no key: provision it first`,
            });
        }

        // decrypt it as a key that never leaves
        const bytes = await this.keyring.decrypt(kept.ciphertext, this.#context(vaultId));
        try {
            return await VaultKey.import(kept.id, bytes);
        } finally {
            bytes.fill(0);
        }
    }

    /** Encrypt every vault key kept under a root key version under the keyring's active one instead, answering how many. */
    async reencrypt(version: number): Promise<number> {
        // read the keys under the version
        const kept = await this.database.select().from(vaultKey);
        const stale = kept.filter((row) => LocalKeyring.version(row.ciphertext) === version);

        // encrypt each under the active version
        for (const row of stale) {
            const context = this.#context(row.vaultId);
            const bytes = await this.keyring.decrypt(row.ciphertext, context);
            try {
                const ciphertext = await this.keyring.encrypt(bytes, context);
                await this.database
                    .update(vaultKey)
                    .set({ ciphertext })
                    .where(eq(vaultKey.vaultId, row.vaultId));
            } finally {
                bytes.fill(0);
            }
        }

        return stale.length;
    }

    /** Seal a vault's key to another host's recipient. */
    async #seal(row: VaultKeyRow, recipient: Recipient): Promise<VaultKeyRow> {
        const bytes = await this.keyring.decrypt(row.ciphertext, this.#context(row.vaultId));
        try {
            return { ...row, ciphertext: await recipient.seal(bytes, transit(row.vaultId)) };
        } finally {
            bytes.fill(0);
        }
    }

    /** Open a vault's key another host sealed to this host's recipient, and encrypt it under this host's keyring. */
    async #open(row: VaultKeyRow, recipient: Recipient): Promise<VaultKeyRow> {
        const bytes = await recipient.open(row.ciphertext, transit(row.vaultId));
        try {
            const ciphertext: Ciphertext = await this.keyring.encrypt(
                bytes,
                this.#context(row.vaultId),
            );

            return { ...row, ciphertext };
        } finally {
            bytes.fill(0);
        }
    }

    /** Encode the encryption context of a vault key at rest: the protocol, the location and the vault. */
    #context(vaultId: string): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(
            JSON.stringify([VAULT_KEY_PROTOCOL, this.location, vaultId]),
        );
    }
}

/** Encode the encryption context of a vault key in transit to another host: the protocol and the vault. */
function transit(vaultId: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(JSON.stringify([VAULT_KEY_PROTOCOL, vaultId]));
}

/** Name the vault a vault resource is. */
function vaultOf(resource: Pick<ResourceRecord, "id">): Identifier<"vault"> {
    return schema.identifier("vault").parse(resource.id);
}
