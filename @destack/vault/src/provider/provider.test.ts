import { Caller } from "@destack/access";
import { space } from "@destack/account/object";
import { type DatabaseConnection, type Table, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { LocalKeyring, Recipient } from "@destack/identity";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { expect, onTestFinished, single, test } from "@destack/test";
import { VaultKeystore } from "../key/index.ts";
import { secret, SecretVersion, secretVersion, vault, vaultKey } from "../object/index.ts";
import { OTHER_MACHINE, VaultFixture, vaultDatabase } from "../test/index.ts";
import { vaultProvider } from "./provider.ts";

/** Copy the rows of some space tables a target lacks, parents first. */
async function copySpace(
    from: DatabaseConnection,
    to: DatabaseConnection,
    tables: readonly Table[],
): Promise<void> {
    for (const table of tables) {
        const rows = await from.select().from(table);
        const written = rows.map((row) =>
            table === secret.table ? { ...row, currentVersion: null } : row,
        );
        if (written.length > 0) {
            await to.insert(table).values(written).onConflictDoNothing();
        }
    }
}

test.each(TEST_DIALECTS)(
    "move a vault's key to a machine with other root keys, sealed to its recipient, and open the secret's value there on %s",
    async (dialect) => {
        // copy a secret value's space rows from the source to the target
        await using fixture = await VaultFixture.open(dialect);
        const { first } = await fixture.createSecret();
        const target = await TestDatabase.create(dialect, vaultDatabase, { isMigrated: true });
        onTestFinished(() => target.close());
        await copySpace(fixture.database, target.database, [space.table, vault.table]);

        // keep the source's and the target's keys under different root keys and locations
        const targetKeyring = await LocalKeyring.import(
            2,
            new Map([[2, crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const source = vaultProvider(fixture.keystore());
        const destination = vaultProvider(
            new VaultKeystore(target.database, targetKeyring, OTHER_MACHINE),
        );

        // seal the key to the target's recipient
        const recipient = await Recipient.generate();
        const started = single(await fixture.database.select().from(vaultKey.table));
        const sealed = await source.rewrap.seal(started, recipient);
        const received = await destination.rewrap.open(sealed, recipient);
        await target.database.insert(vaultKey.table).values(received);
        await copySpace(fixture.database, target.database, [secret.table, secretVersion.table]);

        // open the secret's value on the target with the key version it now has
        const owner = single(await target.database.select().from(secret.table));
        const version = present(
            await SecretVersion.find(Transaction.reading(target.database), first.id, 1),
            "the target's first version",
        );
        const { keyId, ciphertext } = SecretVersion.requireReadable(version, Date.now());
        const keystore = new VaultKeystore(target.database, targetKeyring, OTHER_MACHINE);
        const key = await keystore.open(Transaction.reading(target.database), keyId);
        expect([
            await SecretVersion.open(key, owner, version.id, ciphertext),
            [sealed.ciphertext === started.ciphertext, LocalKeyring.version(received.ciphertext)],
        ]).toEqual([{ encoding: "text", value: "credential" }, [false, 2]]);
    },
);

test.each(TEST_DIALECTS)(
    "destroy a vault with its purged secrets, refusing one with values of live secrets on %s",
    async (dialect) => {
        // create a live secret in the vault
        await using fixture = await VaultFixture.open(dialect);
        const { client, database } = fixture;
        const { key } = await fixture.createSecret();
        const provider = vaultProvider(fixture.keystore());
        const record = {
            id: fixture.vaultId,
            scope: fixture.spaceId,
            spec: {},
            reference: fixture.vaultId,
        };

        // refuse destroying the vault while the secret has a value
        await expect(provider.provision.destroy(record)).rejects.toEqual(
            new ServiceError("CONFLICT", {
                message: `vault has values of secret ${key.id}: purge its secrets first`,
            }),
        );

        // destroy the vault with its key and records once the secret is purged
        const row = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            revision: row.revision,
            requestId: RequestId.create(Date.now()),
        });
        await client.secret.purge({ ...key, requestId: RequestId.create(Date.now()) });
        const machine = fixture.objects();
        const stored = single(await database.select().from(vault.table));
        await machine.mutate(Caller.of(machine.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [vault.call("delete", vault.target(stored))],
        });
        await provider.provision.destroy(record);
        const requested = single(await database.select().from(vault.table));
        await machine.mutate(Caller.of(machine.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [vault.call("finalize", vault.target(requested))],
        });
        const remaining = await Promise.all(
            [vault.table, secret.table, secretVersion.table, vaultKey.table].map((table: Table) =>
                database.select().from(table),
            ),
        );
        expect(remaining).toEqual([[], [], [], []]);
    },
);
