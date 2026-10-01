import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, type Table } from "@destack/db";
import { SystemCall } from "@destack/object/server";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { Recipient } from "@destack/resource";
import { resource, space } from "@destack/space/object";
import { expect, onTestFinished, test } from "@destack/test";
import { LocalKeyring, VaultKey } from "../encryption/index.ts";
import { VaultValue } from "../server/index.ts";
import { secret, secretVersion, vault } from "../object/index.ts";
import { LOCATION, VaultFixture } from "../server/tests/fixture.ts";
import { vaultKey, vaultValue } from "../stack/index.ts";
import { cellTables } from "../server/tests/fixture.ts";
import { vaultProvider } from "./provider.ts";

/** Copy the rows of the zone tables a target lacks, as a transfer's zone copy does. */
async function copyZone(from: DatabaseConnection, to: DatabaseConnection): Promise<void> {
    for (const table of [
        space.table,
        resource.table,
        vault.table,
        secret.table,
        secretVersion.table,
    ]) {
        const rows = await from.select().from(table as Table);
        const written = rows.map((row) =>
            table === secret.table ? { ...row, currentVersion: null } : row,
        );
        if (written.length > 0) {
            await to
                .insert(table as Table)
                .values(written as never)
                .onConflictDoNothing();
        }
    }
}

test.each(TEST_DIALECTS)(
    "copy a vault's values to a target with other root keys, with a version written between the live and fenced passes, on %s",
    async (dialect) => {
        // create a secret version on the source, and copy the zone's rows to the target
        await using fixture = await VaultFixture.open(dialect);
        const { client } = fixture;
        const { first, request } = await fixture.createSecret();
        const target = await TestDatabase.create(dialect, cellTables, { isMigrated: true });
        onTestFinished(() => target.close());
        await copyZone(fixture.database, target.database);

        // keep values under the source's root key in one location, and the target's in another
        const targetKeyring = await LocalKeyring.import(
            "two",
            new Map([["two", crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const source = vaultProvider(fixture.database, await fixture.keyring(), LOCATION);
        const destination = vaultProvider(target.database, targetKeyring, "us");

        // copy live, carrying the cursor into the fenced pass as a transfer does
        const record = {
            id: fixture.vaultId,
            scope: fixture.spaceId,
            kind: "vault" as const,
            spec: {},
            reference: fixture.vaultId,
        };
        const recipient = await Recipient.generate();
        await destination.provision({ ...record, reference: null });
        let cursor: string | undefined;
        const pass = async (stage: "live" | "fenced") => {
            const copy = { record, desired: [], recipient, stage };
            for await (const chunk of source.export(copy, cursor, AbortSignal.timeout(4000))) {
                await destination.import(copy, chunk);
                cursor = chunk.cursor;
            }
        };
        await pass("live");

        // write a second secret version after the live pass, then copy it in the fenced pass
        await client.version.create({ ...request, requestId: RequestId.create() });
        await copyZone(fixture.database, target.database);
        await pass("fenced");

        // read both versions on the target under its own root key, encrypted afresh
        const copied = await target.database.select().from(vaultValue);
        const [targetRow] = await target.database.select().from(vaultKey);
        const targetKey = await VaultKey.load(
            target.database,
            targetKeyring,
            "us",
            targetRow!.vaultId,
        );
        const originals = await fixture.database.select().from(vaultValue);
        const [owned] = await target.database.select().from(secret.table);
        expect([
            await VaultValue.read(target.database, targetKey, owned!, 1),
            await VaultValue.read(target.database, targetKey, owned!, 2),
            copied.map((value) => [value.secretId, value.version, value.keyId]),
            copied.some((value) =>
                originals.some((original) => original.ciphertext === value.ciphertext),
            ),
        ]).toEqual([
            { encoding: "text", value: "credential" },
            { encoding: "text", value: "credential" },
            [
                [first.id, 1, targetKey.id],
                [first.id, 2, targetKey.id],
            ],
            false,
        ]);
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
            kind: "vault" as const,
            spec: {},
            reference: fixture.vaultId,
        };

        // refuse destroying the vault while the secret has a value
        await expect(provider.destroy(record)).rejects.toEqual(
            new ServiceError("CONFLICT", {
                message: `vault has values of secret ${key.id}: purge its secrets first`,
            }),
        );

        // destroy the vault once the secret is purged, deleting its key, its facet and the secret's records
        const row = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            revision: row.revision,
            requestId: RequestId.create(),
        });
        await client.secret.purge({ ...key, requestId: RequestId.create() });
        await provider.destroy(record);
        const [facet] = await database.select().from(vault.table);
        await fixture
            .system(keyring)
            .executeAsSystem(vault, "delete", [SystemCall.of(facet!)], Date.now());
        const remaining = await Promise.all(
            [vault.table, secret.table, secretVersion.table, vaultValue, vaultKey].map((table) =>
                database.select().from(table as Table),
            ),
        );
        expect(remaining).toEqual([[], [], [], [], []]);
    },
);
