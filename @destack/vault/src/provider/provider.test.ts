import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, type Table } from "@destack/db";
import { SystemCall } from "@destack/object/server";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { Recipient } from "@destack/resource";
import { space } from "@destack/space/object";
import { expect, onTestFinished, single, test } from "@destack/test";
import { LocalKeyring } from "@destack/host/keychain";
import { VaultKey } from "../encryption/index.ts";
import { secret, secretVersion, vault, SecretVersion } from "../object/index.ts";
import { LOCATION, VaultFixture } from "../server/tests/fixture.ts";
import { vaultKey } from "../stack/index.ts";
import { cellTables } from "../server/tests/fixture.ts";
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
    "carry a vault's key to a host with other root keys, wrapped for its recipient, and open the secret's value there on %s",
    async (dialect) => {
        // keep a secret value on the source, and copy the zone's rows to the target
        await using fixture = await VaultFixture.open(dialect);
        const { first } = await fixture.createSecret();
        const target = await TestDatabase.create(dialect, cellTables, { isMigrated: true });
        onTestFinished(() => target.close());
        await copyZone(fixture.database, target.database);

        // keep the source's key under its root key in one location, and the target's in another
        const targetKeyring = await LocalKeyring.import(
            "two",
            new Map([["two", crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const source = vaultProvider(fixture.database, await fixture.keyring(), LOCATION);
        const destination = vaultProvider(target.database, targetKeyring, "us");

        // wrap the key for the target's recipient, then under the target's root key
        const recipient = await Recipient.generate();
        const kept = single(await fixture.database.select().from(vaultKey));
        const wrapped = await source.rewrap.wrap(kept, recipient);
        const received = await destination.rewrap.unwrap(wrapped, recipient);
        await target.database.insert(vaultKey).values(received);

        // open the secret's value on the target with the key it now keeps
        const key = await VaultKey.load(target.database, targetKeyring, "us", fixture.vaultId);
        const owner = single(await target.database.select().from(secret.table));
        const version = await SecretVersion.find(target.database, first.id, 1);
        if (version === undefined) {
            throw new TypeError("the target lacks the secret's first version");
        }
        expect([
            await SecretVersion.open(key, owner, version),
            [wrapped.rootKeyId === kept.rootKeyId, received.rootKeyId],
        ]).toEqual([{ encoding: "text", value: "credential" }, [false, "two"]]);
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
        const provider = vaultProvider(database, keyring, LOCATION);
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

        // destroy the vault once the secret is purged, deleting its key, its record and the secret's records
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
