import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, type Table } from "@destack/db";
import { SystemCall } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { Recipient } from "@destack/identity";
import { space } from "@destack/space/object";
import { expect, onTestFinished, single, test } from "@destack/test";
import { LocalKeyring } from "@destack/identity";
import { secret, secretVersion, vault, SecretVersion } from "../object/index.ts";
import { LOCATION, VaultFixture } from "../test/index.ts";
import { vaultKey, vaultDatabase } from "../stack/index.ts";
import { KeyringVaultHost } from "../key/index.ts";
import { vaultProvider } from "./provider.ts";

/** Copy the rows of the zone tables a target lacks, as a transfer's zone copy does. */
async function copyZone(from: DatabaseConnection, to: DatabaseConnection): Promise<void> {
    for (const table of [space.table, vault.table, secret.table, secretVersion.table]) {
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
    "carry a vault's key to a host with other root keys, sealed to its recipient, and open the secret's value there on %s",
    async (dialect) => {
        // copy a secret value's zone rows from the source to the target
        await using fixture = await VaultFixture.open(dialect);
        const { first } = await fixture.createSecret();
        const target = await TestDatabase.create(dialect, vaultDatabase, { isMigrated: true });
        onTestFinished(() => target.close());
        await copyZone(fixture.database, target.database);

        // keep the source's and the target's keys under different root keys and locations
        const targetKeyring = await LocalKeyring.import(
            2,
            new Map([[2, crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const source = vaultProvider(
            new KeyringVaultHost(fixture.database, await fixture.keyring(), LOCATION),
        );
        const destination = vaultProvider(
            new KeyringVaultHost(target.database, targetKeyring, "us"),
        );

        // seal the key to the target's recipient
        const recipient = await Recipient.generate();
        const kept = single(await fixture.database.select().from(vaultKey));
        const sealed = await source.rewrap.seal(kept, recipient);
        const received = await destination.rewrap.open(sealed, recipient);
        await target.database.insert(vaultKey).values(received);

        // open the secret's value on the target with the key it now keeps
        const key = await new KeyringVaultHost(target.database, targetKeyring, "us").key(
            target.database,
            fixture.vaultId,
        );
        const owner = single(await target.database.select().from(secret.table));
        const version = await SecretVersion.find(target.database, first.id, 1);
        if (version === undefined) {
            throw new TypeError("the target lacks the secret's first version");
        }
        expect([
            await SecretVersion.decrypt(key, owner, version),
            [sealed.ciphertext === kept.ciphertext, LocalKeyring.version(received.ciphertext)],
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
        const keyring = await fixture.keyring();
        const provider = vaultProvider(new KeyringVaultHost(database, keyring, LOCATION));
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
            requestId: RequestId.create(),
        });
        await client.secret.purge({ ...key, requestId: RequestId.create() });
        const cell = await fixture.cell(keyring);
        const stored = single(await database.select().from(vault.table));
        await cell.executeAsSystem(vault, "delete", [SystemCall.of(stored)], Date.now());
        await provider.provision.destroy(record);
        const requested = single(await database.select().from(vault.table));
        await cell.executeAsSystem(vault, "finalize", [SystemCall.of(requested)], Date.now());
        const remaining = await Promise.all(
            [vault.table, secret.table, secretVersion.table, vaultKey].map((table: Table) =>
                database.select().from(table),
            ),
        );
        expect(remaining).toEqual([[], [], [], []]);
    },
);
