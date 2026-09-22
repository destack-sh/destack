import { writeVersion, readVersion } from "../../secret/version.ts";
import { expect, test } from "@destack/test";
import { createRequestId } from "@destack/service/request";
import { eq } from "@destack/db";
import { space } from "@destack/model/regional";
import { connect } from "../../secret/client.ts";
import { AuditOutbox } from "@destack/audit/outbox";
import { AuditRecorder } from "@destack/audit";
import { VaultFixture } from "./fixture.ts";
import { vaultValue } from "../../stack/index.ts";
import { LocalKeyring, EnvelopeEncryption } from "../../encryption/index.ts";
import { Vault } from "../../vault/index.ts";
import { vaultPackage } from "../../audit/index.ts";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as turso from "@destack/db/turso";
import { vaultSchema } from "../../stack/index.ts";

test("reject ciphertext copied between secrets", async () => {
    await using fixture = await VaultFixture.open();
    const { client, database, spaceId, vaultId } = fixture;
    const { first, request } = await fixture.createSecret();
    const second = await client.secret.create({
        spaceId,
        vaultId,
        name: "second",
        requestId: createRequestId(),
    });
    await client.version.write({ ...request, secretId: second.id, requestId: createRequestId() });

    // a ciphertext copied to another secret fails authenticated decryption
    const rows = await database.select().from(vaultValue);
    const original = rows.find((row) => row.secretId === first.id)!;
    await database
        .update(vaultValue)
        .set({
            ciphertext: original.ciphertext,
            nonce: original.nonce,
            wrappedKey: original.wrappedKey,
            keyNonce: original.keyNonce,
        })
        .where(eq(vaultValue.secretId, second.id));
    await expect(client.version.read({ spaceId, secretId: second.id })).rejects.toMatchObject({
        code: "INTERNAL_SERVER_ERROR",
    });
});

test("rotate root keys and reopen without the old key", async () => {
    await using fixture = await VaultFixture.open();
    const { database, spaceId } = fixture;
    const { key, request, written } = await fixture.createSecret();

    // rewrap under a new root while keeping the same value version
    const before = await database.select().from(vaultValue).get();
    const next = crypto.getRandomValues(new Uint8Array(32));
    const keys = await LocalKeyring.import(
        "two",
        new Map([
            ["one", fixture.root],
            ["two", next],
        ]),
    );
    const rotating = new Vault(database, new EnvelopeEncryption(keys), "eu");
    const server = await fixture.host(rotating);
    const client = connect({
        url: "http://vault.test",
        headers: { Authorization: `Bearer ${fixture.userId}` },
        fetch: (request) => server.fetch(request),
    });
    await client.version.rewrap({
        ...key,
        version: 1,
        revision: written.secret.revision,
        requestId: createRequestId(),
    });
    const batch = { spaceId, keyId: "one", limit: 100 };
    expect(await client.request.rewrap(batch)).toBe(2);
    expect(await client.request.rewrap(batch)).toBe(0);
    const after = await database.select().from(vaultValue).get();
    expect({ ciphertext: after!.ciphertext, nonce: after!.nonce, version: after!.version }).toEqual(
        {
            ciphertext: before!.ciphertext,
            nonce: before!.nonce,
            version: before!.version,
        },
    );
    expect(after!.keyId).toBe("two");

    // retain value access and exact mutation replay after retiring the old root
    const currentKeys = await LocalKeyring.import("two", new Map([["two", next]]));
    const current = new Vault(database, new EnvelopeEncryption(currentKeys), "eu");
    expect((await readVersion(current, key, fixture.context)).value).toEqual(request.value);
    expect(await writeVersion(current, request, fixture.context)).toEqual(written);
});

test("recover persisted values and exact retries after reopening the database", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-vault-"));
    try {
        // commit a value, then close the original connection and discard its service state
        const file = join(directory, "vault.db");
        const fixture = await VaultFixture.open(file);
        let saved: Awaited<ReturnType<VaultFixture["createSecret"]>>;
        try {
            saved = await fixture.createSecret();
        } finally {
            await fixture.close();
        }

        // reconstruct storage using only persisted records and separately retained keys
        const database = await turso.connect(file, vaultSchema);
        try {
            const keys = await LocalKeyring.import("one", new Map([["one", fixture.root]]));
            const vault = new Vault(database, new EnvelopeEncryption(keys), "eu");
            const owner = await database
                .select()
                .from(space)
                .where(eq(space.id, fixture.spaceId))
                .get();
            const context = {
                audience: vaultPackage.id,
                caller: fixture.context.caller,
                audit: new AuditRecorder(
                    {
                        actor: { type: "user", id: fixture.userId },
                        delegation: [],
                        package: vaultPackage,
                        service: "vault",
                        spaceId: fixture.spaceId,
                        accountId: owner!.accountId,
                    },
                    new AuditOutbox(database),
                ),
            };
            expect(await readVersion(vault, saved.key, context)).toEqual({
                version: saved.written.version,
                value: saved.request.value,
            });
            expect(await writeVersion(vault, saved.request, context)).toEqual(saved.written);

            // reject unavailable keys without creating replacement material
            const replacement = crypto.getRandomValues(new Uint8Array(32));
            const missing = await LocalKeyring.import("two", new Map([["two", replacement]]));
            const inaccessible = new Vault(database, new EnvelopeEncryption(missing), "eu");
            await expect(readVersion(inaccessible, saved.key, context)).rejects.toMatchObject({
                code: "KEY_UNAVAILABLE",
                message: "required root key is unavailable",
            });
        } finally {
            await database.close();
        }
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test("withhold plaintext when audit persistence fails", async () => {
    await using fixture = await VaultFixture.open();
    const { key } = await fixture.createSecret();

    // no plaintext result escapes when durable audit persistence fails
    const audit = new AuditRecorder(
        {
            actor: { type: "user", id: fixture.userId },
            delegation: [],
            package: vaultPackage,
            service: "vault",
        },
        {
            append: async () => {
                throw new Error("audit unavailable");
            },
        },
    );
    await expect(readVersion(fixture.vault, key, { ...fixture.context, audit })).rejects.toThrow(
        "audit unavailable",
    );
});
