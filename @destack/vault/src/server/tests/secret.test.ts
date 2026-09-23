import { readVersion } from "../../secret/version.ts";
import { expect, test } from "@destack/test";
import { createRequestId } from "@destack/service/request";
import { and, eq } from "@destack/db";
import { secret } from "@destack/model/space";
import { AuditOutbox } from "@destack/audit/outbox";
import { VaultFixture } from "./fixture.ts";
import { vaultValue } from "../../stack/index.ts";
import { versionRead, vaultPackage } from "../../audit/index.ts";

/** Exercise versioned secret access and recoverable deletion through typed HTTP. */
test("roundtrip bounded text and binary values without committing rejected writes", async () => {
    await using fixture = await VaultFixture.open();
    const { client, spaceId, vaultId } = fixture;
    const created = await client.secret.create({
        spaceId,
        vaultId,
        name: "binary",
        requestId: createRequestId(),
    });
    const key = { spaceId, secretId: created.id };
    const value = {
        encoding: "base64" as const,
        value: Uint8Array.from({ length: 65536 }, (_, index) => index % 256).toBase64(),
    };
    const written = await client.version.write({
        ...key,
        revision: created.revision,
        requestId: createRequestId(),
        value,
    });
    expect(await client.version.read(key)).toEqual({ version: written.version, value });

    // enforce the decoded byte limit for both transport encodings
    for (const rejected of [
        { encoding: "base64" as const, value: new Uint8Array(65537).toBase64() },
        { encoding: "text" as const, value: "é".repeat(32769) },
    ]) {
        await expect(
            client.version.write({
                ...key,
                revision: written.secret.revision,
                requestId: createRequestId(),
                value: rejected,
            }),
        ).rejects.toMatchObject({ code: "PAYLOAD_TOO_LARGE" });
    }
    await expect(
        client.version.write({
            ...key,
            revision: written.secret.revision,
            requestId: createRequestId(),
            value: { encoding: "base64", value: "!" },
        }),
    ).rejects.toMatchObject({ code: "BAD_REQUEST" });
    expect(await client.secret.get(key)).toEqual(written.secret);
    expect(await client.version.list(key)).toEqual({ items: [written.version], cursor: null });

    // accept the exact UTF-8 limit and preserve the previous immutable binary version
    const text = { encoding: "text" as const, value: "é".repeat(32768) };
    const next = await client.version.write({
        ...key,
        revision: written.secret.revision,
        requestId: createRequestId(),
        value: text,
    });
    expect(await client.version.read(key)).toEqual({ version: next.version, value: text });
    expect(await client.version.read({ ...key, version: 1 })).toEqual({
        version: written.version,
        value,
    });
});

test("manage secret versions and recover deleted secrets", async () => {
    const fixture = await VaultFixture.open();
    const { client, database, spaceId, vaultId } = fixture;
    try {
        const created = await client.secret.create({
            spaceId,
            vaultId,
            requestId: createRequestId(),
            name: "github",
            tags: {},
        });
        const key = { spaceId, secretId: created.id };
        expect(await client.secret.get(key)).toEqual(created);
        expect(await client.secret.list({ spaceId, vaultId })).toEqual({
            items: [created],
            cursor: null,
        });

        // retain one immutable version across retries and reject altered retry contents
        const request = {
            ...key,
            requestId: createRequestId(),
            revision: created.revision,
            value: { encoding: "text" as const, value: "oauth-refresh-token" },
            promote: true,
        };
        const written = await client.version.write(request);
        expect(await client.version.write(request)).toEqual(written);
        await expect(
            client.version.write({ ...request, value: { encoding: "text", value: "different" } }),
        ).rejects.toMatchObject({ code: "CONFLICT" });
        expect(await client.version.read(key)).toEqual({
            version: written.version,
            value: request.value,
        });
        expect(await client.version.list(key)).toEqual({ items: [written.version], cursor: null });

        // stage a second version before atomically selecting it
        const staged = await client.version.write({
            ...key,
            requestId: createRequestId(),
            revision: written.secret.revision,
            value: { encoding: "base64", value: "AAECAw==" },
            promote: false,
        });
        expect((await client.version.read(key)).value).toEqual(request.value);
        const promoted = await client.version.promote({
            ...key,
            requestId: createRequestId(),
            revision: staged.secret.revision,
            version: 2,
        });
        expect((await client.version.read(key)).value).toEqual({
            encoding: "base64",
            value: "AAECAw==",
        });

        // preserve explicit disabling across deletion and restoration
        const disabled = await client.secret.disable({
            ...key,
            requestId: createRequestId(),
            revision: promoted.revision,
        });
        await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
        const deleted = await client.secret.delete({
            ...key,
            requestId: createRequestId(),
            revision: disabled.revision,
        });
        const restored = await client.secret.restore({
            ...key,
            requestId: createRequestId(),
            revision: deleted.revision,
        });
        await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
        const enabled = await client.secret.enable({
            ...key,
            requestId: createRequestId(),
            revision: restored.revision,
        });
        expect((await client.version.read(key)).version.version).toBe(2);

        // retain destroyed version metadata and reject further value access
        const destroyed = await client.version.destroy({
            ...key,
            requestId: createRequestId(),
            revision: enabled.revision,
            version: 1,
        });
        expect(destroyed.destroyedAt).not.toBeNull();
        await expect(client.version.read({ ...key, version: 1 })).rejects.toMatchObject({
            code: "FORBIDDEN",
        });
        const ciphertext = await database.select().from(vaultValue);
        expect(ciphertext.map((row) => row.version)).toEqual([2]);

        // durably record disclosure without including plaintext in any event
        const events = await new AuditOutbox(database).read(1000);
        const actions = events.filter((event) => event.action.package.id === vaultPackage.id);
        expect(actions.map((event) => event.action.name)).toEqual([
            "secret.create",
            "version.write",
            "version.read",
            "version.write",
            "version.read",
            "version.promote",
            "version.read",
            "secret.disable",
            "secret.delete",
            "secret.restore",
            "secret.enable",
            "version.read",
            "version.destroy",
        ]);
        const reads = events.filter((event) => event.action.name === versionRead.name);
        expect(reads.map((event) => event.details)).toEqual([
            { version: 1 },
            { version: 1 },
            { version: 2 },
            { version: 2 },
        ]);
        expect(events.some((event) => JSON.stringify(event).includes("oauth-refresh-token"))).toBe(
            false,
        );
    } finally {
        await fixture.close();
    }
});

test("purge expired recovery records exactly once", async () => {
    await using fixture = await VaultFixture.open();
    const { client, database } = fixture;
    const { first, key } = await fixture.createSecret();

    // finalize deletion once and retain its tombstone
    const row = await client.secret.get(key);
    await client.secret.delete({
        ...key,
        revision: row.revision,
        requestId: createRequestId(),
    });
    await database
        .update(secret)
        .set({ deleteAt: Date.now() - 1 })
        .where(eq(secret.id, first.id));
    await fixture.allowRead(false);
    expect(await client.secret.purge({ spaceId: fixture.spaceId, limit: 100 })).toBe(1);
    expect(await client.secret.purge({ spaceId: fixture.spaceId, limit: 100 })).toBe(0);

    // reject unauthorized maintenance even when no records remain due
    await fixture.allow(false);
    await expect(
        client.secret.purge({ spaceId: fixture.spaceId, limit: 100 }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
    });
    expect(
        await database
            .select()
            .from(vaultValue)
            .where(and(eq(vaultValue.secretId, first.id), eq(vaultValue.version, 1))),
    ).toEqual([]);
    await fixture.allow(true);
    await fixture.allowRead(true);
    await expect(readVersion(fixture.vault, key, fixture.context)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "secret is unavailable",
    });
});
