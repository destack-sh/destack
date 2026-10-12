import {
    and,
    type DatabaseConnection,
    eq,
    isNotNull,
    type Select,
    type Transaction,
} from "@destack/db";
import { type Ciphertext, type Keyring, LocalKeyring, type Recipient } from "@destack/identity";
import type { Provisioner, ResourceRecord, Rewrapper } from "@destack/resource";
import { type Identifier, present, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { VAULT_PROVIDER, VaultKind } from "../declare/kind.ts";
import { secret, secretVersion, vaultKey } from "../object/index.ts";
import { vaultService } from "../service/index.ts";
import { VAULT_KEY_BYTES, VaultKey } from "./key.ts";

/** The protocol a vault key's encryption context starts with, before machine, vault and version. */
const VAULT_KEY_PROTOCOL = "@destack/vault/key";

/** A vault key version as its table keeps it. */
type VaultKeyRow = Select<typeof vaultKey.table>;

/** The vault and identifier of a key version, which its encryption contexts include. */
type KeyAddress = { readonly parentId: string; readonly id: string };

/** A machine's vault keys, encrypted under its keyring. */
export class VaultKeystore implements Provisioner<typeof VaultKind> {
    /** The provider code of vaults, which their declarations' connectors use. */
    readonly provider = VAULT_PROVIDER;
    /** The vault service's database keeping the vaults' keys and secrets. */
    readonly database: DatabaseConnection;
    /** The machine's keyring encrypting the vaults' keys. */
    readonly keyring: Keyring;
    /** The machine the vaults' keys belong to, which their encryption context includes. */
    readonly machine: Identifier<"machine">;
    /** The sealing of the keys to the machine a space moves to. */
    readonly rewrap: Rewrapper<typeof vaultKey.table, VaultKeyRow>;

    /** Open vaults' keys in the vault service's database under a machine's keyring. */
    constructor(database: DatabaseConnection, keyring: Keyring, machine: Identifier<"machine">) {
        // store the database, keyring and machine, and seal keys with them
        this.database = database;
        this.keyring = keyring;
        this.machine = machine;
        this.rewrap = {
            table: vaultKey.table,
            seal: (row, recipient) => this.#seal(row, recipient),
            open: (row, recipient) => this.#open(row, recipient),
        };
    }

    /** Answer the vault service's address for a new vault. */
    async provision(): Promise<{ reference: string }> {
        return { reference: vaultService.package.name };
    }

    /** Refuse destroying a vault that keeps secret values, its key versions going with its row. */
    async destroy(resource: ResourceRecord<typeof VaultKind>): Promise<void> {
        // find a secret value of the vault
        const vaultId = schema.identifier("vault").parse(resource.id);
        const [stored] = await this.database
            .select({ secretId: secretVersion.table.parentId })
            .from(secretVersion.table)
            .innerJoin(secret.table, eq(secret.table.id, secretVersion.table.parentId))
            .where(
                and(eq(secret.table.parentId, vaultId), isNotNull(secretVersion.table.ciphertext)),
            )
            .limit(1);

        // refuse while one is left
        if (stored !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `vault has values of secret ${stored.secretId}: purge its secrets first`,
            });
        }
    }

    /** Generate a key version's material, encrypted under the keyring for its vault and version. */
    async generate(vaultId: Identifier<"vault">, keyId: string): Promise<Ciphertext> {
        const bytes = crypto.getRandomValues(new Uint8Array(VAULT_KEY_BYTES));
        try {
            return await this.keyring.encrypt(
                bytes,
                this.#context({ parentId: vaultId, id: keyId }),
            );
        } finally {
            bytes.fill(0);
        }
    }

    /** Open a vault's newest key version, which encrypts its new values. */
    async latest(transaction: Transaction, vaultId: Identifier<"vault">): Promise<VaultKey> {
        const [row] = await transaction.rows(
            vaultKey.table,
            { parentId: vaultId },
            { orderBy: { number: "desc" }, limit: 1 },
        );

        return this.#import(present(row, `the key of vault ${vaultId}`));
    }

    /** Open one key version, which decrypts the values it encrypted. */
    async open(transaction: Transaction, keyId: string): Promise<VaultKey> {
        const id = schema.identifier("vault-key").parse(keyId);
        const row = await transaction.row(vaultKey.table, { id });

        return this.#import(present(row, `vault key ${keyId}`));
    }

    /** Encrypt the key versions of a root key version under the active one, counting them. */
    async reencrypt(version: number): Promise<number> {
        // read the key versions under the root key version
        const rows = await this.database.select().from(vaultKey.table);
        const stale = rows.filter((row) => LocalKeyring.version(row.ciphertext) === version);

        // encrypt each under the active version
        for (const row of stale) {
            const ciphertext = await this.#decrypt(row, (bytes) =>
                this.keyring.encrypt(bytes, this.#context(row)),
            );
            await this.database
                .update(vaultKey.table)
                .set({ ciphertext })
                .where(eq(vaultKey.table.id, row.id));
        }

        return stale.length;
    }

    /** Decrypt a key version's material as a key that never leaves. */
    #import(row: VaultKeyRow): Promise<VaultKey> {
        return this.#decrypt(row, (bytes) => VaultKey.import(row.id, bytes));
    }

    /** Seal a key version to another machine's recipient. */
    async #seal(row: VaultKeyRow, recipient: Recipient): Promise<VaultKeyRow> {
        const ciphertext = await this.#decrypt(row, (bytes) => recipient.seal(bytes, transit(row)));

        return { ...row, ciphertext };
    }

    /** Open a key version another machine sealed to this machine's recipient. */
    async #open(row: VaultKeyRow, recipient: Recipient): Promise<VaultKeyRow> {
        const bytes = await recipient.open(row.ciphertext, transit(row));
        try {
            return { ...row, ciphertext: await this.keyring.encrypt(bytes, this.#context(row)) };
        } finally {
            bytes.fill(0);
        }
    }

    /** Use a key version's material, erasing it after. */
    async #decrypt<Result>(
        row: VaultKeyRow,
        use: (bytes: Uint8Array<ArrayBuffer>) => Promise<Result>,
    ): Promise<Result> {
        const bytes = await this.keyring.decrypt(row.ciphertext, this.#context(row));
        try {
            return await use(bytes);
        } finally {
            bytes.fill(0);
        }
    }

    /** Encode the encryption context of a key version at rest. */
    #context(row: KeyAddress): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(
            JSON.stringify([VAULT_KEY_PROTOCOL, this.machine, row.parentId, row.id]),
        );
    }
}

/** Encode the encryption context of a key version in transit to another machine. */
function transit(row: KeyAddress): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(JSON.stringify([VAULT_KEY_PROTOCOL, row.parentId, row.id]));
}
