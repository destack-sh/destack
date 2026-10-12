import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { eq, isNotNull } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { present } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { expect, single, test } from "@destack/test";
import { secretVersion, vaultKey } from "../object/index.ts";
import { VaultFixture } from "../test/index.ts";

test.each(TEST_DIALECTS)("refuse ciphertext copied between secrets on %s", async (dialect) => {
    await using fixture = await VaultFixture.open(dialect);
    const { client, database, spaceId, vaultId } = fixture;
    const { first, request } = await fixture.createSecret();
    const second = await client.secret.create({
        scope: spaceId,
        parentId: vaultId,
        name: "second",
        requestId: RequestId.create(Date.now()),
    });
    await client.version.create({
        ...request,
        parentId: second.id,
        requestId: RequestId.create(Date.now()),
    });

    // fail authenticated decryption of a ciphertext copied to another secret
    const rows = await database
        .select({
            secretId: secretVersion.table.parentId,
            version: secretVersion.table.number,
            ciphertext: secretVersion.table.ciphertext,
        })
        .from(secretVersion.table)
        .where(isNotNull(secretVersion.table.ciphertext));
    const original = single(rows.filter((row) => row.secretId === first.id));
    await database
        .update(secretVersion.table)
        .set({ ciphertext: original.ciphertext })
        .where(eq(secretVersion.table.parentId, second.id));
    await expect(client.secret.read({ scope: spaceId, id: second.id })).rejects.toEqual(
        new ServiceError("INTERNAL_SERVER_ERROR", { message: "internal server error" }),
    );
});

test.each(TEST_DIALECTS)(
    "reencrypt the vault keys under a new root key, keeping every value as stored, before retiring the old one on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { database } = fixture;
        const keychain = new MemoryKeychain();
        const name = "vault/machine-1";

        // keep the vault's key and a value under the keychain's first root key
        const first = await LocalKeyring.open(keychain, name);
        const [started] = await database.select().from(vaultKey.table);
        const keyId = present(started, "the vault's first key").id;
        const ciphertext = await fixture.keystore(first).generate(fixture.vaultId, keyId);
        await database.update(vaultKey.table).set({ ciphertext });
        fixture.client = fixture.connect(fixture.serve(first));
        const { key, request } = await fixture.createSecret();
        const stored = await database
            .select({
                secretId: secretVersion.table.parentId,
                version: secretVersion.table.number,
                ciphertext: secretVersion.table.ciphertext,
            })
            .from(secretVersion.table)
            .where(isNotNull(secretVersion.table.ciphertext));

        // reencrypt the vault key under a new root key before retiring the old one
        const rotated = await LocalKeyring.rotate(keychain, name);
        const reencrypted = await fixture.keystore(rotated).reencrypt(first.active);
        const retired = await LocalKeyring.retire(keychain, name, first.active);

        // read the untouched value under the remaining root key
        const current = fixture.connect(fixture.serve(retired));
        expect([
            reencrypted,
            (await database.select().from(vaultKey.table)).map((row) =>
                LocalKeyring.version(row.ciphertext),
            ),
            await database
                .select({
                    secretId: secretVersion.table.parentId,
                    version: secretVersion.table.number,
                    ciphertext: secretVersion.table.ciphertext,
                })
                .from(secretVersion.table)
                .where(isNotNull(secretVersion.table.ciphertext)),
            await current.secret.read(key),
        ]).toEqual([1, [rotated.active], stored, { version: 1, value: request.value }]);
    },
);

test("read persisted values after reopening the database, and refuse without their root key", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-vault-"));
    try {
        // commit a value before closing the database
        const file = join(directory, "vault.db");
        const original = await VaultFixture.openFile(file);
        let saved;
        try {
            saved = await original.createSecret();
        } finally {
            await original.close();
        }

        // reopen the database and read the value under the retained root key
        await using reopened = await VaultFixture.openFile(file, original);
        expect(await reopened.client.secret.read(saved.key)).toEqual({
            version: 1,
            value: saved.request.value,
        });

        // refuse reading without the root key protecting the value
        const replacement = crypto.getRandomValues(new Uint8Array(32));
        const missing = reopened.connect(
            reopened.serve(await LocalKeyring.import(2, new Map([[2, replacement]]))),
        );
        await expect(missing.secret.read(saved.key)).rejects.toEqual(
            new ServiceError("INTERNAL_SERVER_ERROR", { message: "internal server error" }),
        );
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
