import { TEST_DIALECTS } from "@destack/db/test";
import { testCallKey } from "@destack/service/test";
import { Journal } from "@destack/audit";
import { eq, isNotNull } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { identifier } from "@destack/schema";
import { binding, installation } from "@destack/space/object";
import { v7 } from "uuid";
import { secret, secretVersion } from "../../object/index.ts";
import { SPACE, VAULT, VaultFixture } from "./fixture.ts";

/** The recovery window the fixture's host gives deleted secrets, in milliseconds. */
const RECOVERY_MILLISECONDS = 30 * 86_400_000;

/** How long the purge controller may take to purge an expired secret, far above its one-commit reaction. */
const PURGE_WAIT_MILLISECONDS = 5000;

test.each(TEST_DIALECTS)(
    "roundtrip bounded text and binary values without committing rejected writes on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client, spaceId, vaultId } = fixture;
        const created = await client.secret.create({
            spaceId,
            parentId: vaultId,
            name: "binary",
            requestId: RequestId.create(),
        });
        const key = { spaceId, id: created.id };
        const value = {
            encoding: "base64" as const,
            value: Uint8Array.from({ length: 65536 }, (_, index) => index % 256).toBase64(),
        };
        const written = await client.version.create({
            spaceId,
            parentId: created.id,
            requestId: RequestId.create(),
            value,
        });
        expect(await client.secret.read(key)).toEqual({ version: 1, value });

        // enforce the decoded byte limit for both transport encodings
        for (const rejected of [
            { encoding: "base64" as const, value: new Uint8Array(65537).toBase64() },
            { encoding: "text" as const, value: "é".repeat(32769) },
        ]) {
            await expect(
                client.version.create({
                    spaceId,
                    parentId: created.id,
                    requestId: RequestId.create(),
                    value: rejected,
                }),
            ).rejects.toEqual(
                new ServiceError("PAYLOAD_TOO_LARGE", {
                    message: "secret value exceeds 65536 bytes",
                }),
            );
        }
        expect(
            await client.version
                .create({
                    spaceId,
                    parentId: created.id,
                    requestId: RequestId.create(),
                    value: { encoding: "base64", value: "!" },
                })
                .catch((error: { code: string; message: string }) => [error.code, error.message]),
        ).toEqual(["BAD_REQUEST", "invalid input: value.value: invalid base64-encoded string"]);
        const versions = await client.version.list({ spaceId });
        expect(versions.items).toEqual([written]);

        // accept the exact UTF-8 limit and keep the earlier binary version readable
        const text = { encoding: "text" as const, value: "é".repeat(32768) };
        await client.version.create({
            spaceId,
            parentId: created.id,
            requestId: RequestId.create(),
            value: text,
        });
        expect(await client.secret.read(key)).toEqual({ version: 2, value: text });
        expect(await client.secret.read({ ...key, version: 1 })).toEqual({ version: 1, value });
    },
);

test.each(TEST_DIALECTS)(
    "manage secret versions and recover deleted secrets on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client, database, spaceId, vaultId } = fixture;
        const created = await client.secret.create({
            spaceId,
            parentId: vaultId,
            requestId: RequestId.create(),
            name: "github",
        });
        const key = { spaceId, id: created.id };
        expect(await client.secret.get(key)).toEqual(created);

        // keep one version across retries of the same request
        const request = {
            spaceId,
            parentId: created.id,
            requestId: RequestId.create(),
            value: { encoding: "text" as const, value: "oauth-refresh-token" },
            promote: true,
        };
        const written = await client.version.create(request);
        expect(await client.version.create(request)).toEqual(written);

        // refuse a retry changing the secret value or what the journal fingerprints plainly
        await expect(
            client.version.create({ ...request, value: { encoding: "text", value: "different" } }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", {
                defined: true,
                message: "request identifier has already been used",
            }),
        );
        await expect(client.version.create({ ...request, promote: false })).rejects.toEqual(
            new ServiceError("CONFLICT", {
                defined: true,
                message: "request identifier has already been used",
            }),
        );
        expect(await client.secret.read(key)).toEqual({ version: 1, value: request.value });

        // stage a second version before selecting it as current
        const staged = await client.version.create({
            spaceId,
            parentId: created.id,
            requestId: RequestId.create(),
            value: { encoding: "base64", value: "AAECAw==" },
            promote: false,
        });
        expect(await client.secret.read(key)).toEqual({ version: 1, value: request.value });
        const promoted = await client.secret.promote({
            ...key,
            requestId: RequestId.create(),
            version: staged.number,
        });
        expect(promoted).toEqual({ ...(await client.secret.get(key)), currentVersion: 2 });
        expect(await client.secret.read(key)).toEqual({
            version: 2,
            value: { encoding: "base64", value: "AAECAw==" },
        });

        // keep explicit disabling across deletion and restoration
        await client.secret.disable({ ...key, requestId: RequestId.create() });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "secret is unavailable" }),
        );
        const disabled = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            requestId: RequestId.create(),
            revision: disabled.revision,
        });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is in the trash" }),
        );
        await client.secret.restore({ ...key, requestId: RequestId.create() });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "secret is unavailable" }),
        );
        await client.secret.enable({ ...key, requestId: RequestId.create() });
        expect((await client.secret.read(key)).version).toBe(2);

        // keep a destroyed version's record and refuse its value
        const destroyed = await client.version.destroy({
            spaceId,
            id: written.id,
            requestId: RequestId.create(),
        });
        expect(destroyed).toEqual({
            ...written,
            destroyedAt: expect.any(Number),
            revision: 2,
            updatedAt: expect.any(Number),
        });
        await expect(client.secret.read({ ...key, version: 1 })).rejects.toEqual(
            new ServiceError("FORBIDDEN", {
                defined: true,
                message: "secret version is unavailable",
            }),
        );
        await expect(
            client.version.disable({ spaceId, id: written.id, requestId: RequestId.create() }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret version is destroyed" }),
        );
        const ciphertext = await database
            .select({
                secretId: secretVersion.table.parentId,
                version: secretVersion.table.number,
                envelope: secretVersion.table.envelope,
            })
            .from(secretVersion.table)
            .where(isNotNull(secretVersion.table.envelope));
        expect(ciphertext.map((row) => row.version)).toEqual([2]);

        // record every change, disclosure and refused change, the system's included, and no plaintext in any call
        const calls = await new Journal(database, testCallKey).read({ limit: 1000 });
        const actions = calls.filter(
            (call) =>
                call.execution!.context.package.id === SPACE.id &&
                call.execution!.category !== "denial",
        );
        expect(actions.map((call) => call.method)).toEqual([
            "secret.create",
            "secret.get",
            "secret.select",
            "version.create",
            "secret.read",
            "version.create",
            "secret.read",
            "secret.promote",
            "secret.get",
            "secret.read",
            "secret.disable",
            "secret.get",
            "secret.delete",
            "secret.read",
            "secret.restore",
            "secret.enable",
            "secret.read",
            "version.destroy",
            "version.disable",
        ]);

        // record the version each read disclosed, and none for the read of a trashed secret
        const reads = actions.filter((call) => call.method === "secret.read");
        expect(reads.map((call) => call.execution!.details)).toEqual([
            { version: 1 },
            { version: 1 },
            { version: 2 },
            {},
            { version: 2 },
        ]);

        // record each refused read once, as its procedure's denial
        const denials = calls.filter((call) => call.execution!.category === "denial");
        expect(denials.map((call) => [call.execution!.targets, call.execution!.outcome])).toEqual(
            ["secret is unavailable", "secret is unavailable", "secret version is unavailable"].map(
                (message) => [
                    { procedure: { type: "procedure", id: "secret.read" } },
                    { kind: "denied", error: { code: "FORBIDDEN", status: 403, message } },
                ],
            ),
        );

        // leave the plaintext out of every call the journal keeps
        const disclosures = JSON.stringify(calls).split("oauth-refresh-token").length - 1;
        expect(disclosures).toBe(0);
    },
);

test.each(TEST_DIALECTS)(
    "refuse writing a version with a past expiry and reading an expired version on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client, database } = fixture;
        const { key, request } = await fixture.createSecret();

        // refuse a version that expires before it is written
        await expect(
            client.version.create({
                ...request,
                requestId: RequestId.create(),
                expiresAt: Date.now() - 1000,
            }),
        ).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "secret expiry must be in the future",
            }),
        );

        // read a version before its expiry
        const expiring = await client.version.create({
            ...request,
            requestId: RequestId.create(),
            expiresAt: Date.now() + 60_000,
        });
        expect(await client.secret.read(key)).toEqual({ version: 2, value: request.value });

        // refuse reading the version once its expiry passed
        await database
            .update(secretVersion.table)
            .set({ createdAt: Date.now() - 2000, expiresAt: Date.now() - 1000 })
            .where(eq(secretVersion.table.id, expiring.id));
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", {
                defined: true,
                message: "secret version is unavailable",
            }),
        );
    },
);

test.each(TEST_DIALECTS)(
    "purge a deleted secret at once or when its window ends, destroying its values and keeping its records on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client, database, spaceId, vaultId } = fixture;
        const { first, key, written } = await fixture.createSecret();

        // refuse purging before deletion
        await expect(
            client.secret.purge({ ...key, requestId: RequestId.create() }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is not deleted" }),
        );
        const row = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            revision: row.revision,
            requestId: RequestId.create(),
        });

        // purge another deleted secret at once, within its window, with the purge permission
        const other = await client.secret.create({
            spaceId,
            parentId: vaultId,
            name: "other",
            requestId: RequestId.create(),
        });
        const otherKey = { spaceId, id: other.id };
        await client.secret.delete({
            ...otherKey,
            revision: other.revision,
            requestId: RequestId.create(),
        });
        expect(await client.secret.purge({ ...otherKey, requestId: RequestId.create() })).toEqual(
            {},
        );
        expect((await client.secret.get(otherKey)).purgedAt).toEqual(expect.any(Number));

        // destroy every value after the window ends and keep the secret and its versions as records
        await database
            .update(secret.table)
            .set({ deletionRequestedAt: Date.now() - RECOVERY_MILLISECONDS })
            .where(eq(secret.table.id, first.id));
        const isPurged = await database.log.until(async () => {
            const [row] = await database
                .select({ purgedAt: secret.table.purgedAt })
                .from(secret.table)
                .where(eq(secret.table.id, first.id));

            return row!.purgedAt !== null;
        }, AbortSignal.timeout(PURGE_WAIT_MILLISECONDS));
        expect(isPurged).toBe(true);
        expect(
            await database
                .select({
                    secretId: secretVersion.table.parentId,
                    version: secretVersion.table.number,
                    envelope: secretVersion.table.envelope,
                })
                .from(secretVersion.table)
                .where(isNotNull(secretVersion.table.envelope)),
        ).toEqual([]);
        const [version] = await database
            .select()
            .from(secretVersion.table)
            .where(eq(secretVersion.table.id, written.id));
        expect(version!.destroyedAt).toEqual(expect.any(Number));
        const [purged] = await database
            .select()
            .from(secret.table)
            .where(eq(secret.table.id, first.id));
        expect(purged!.purgedAt).toEqual(expect.any(Number));

        // refuse purging, restoring and reading a purged secret
        await expect(
            client.secret.purge({ ...key, requestId: RequestId.create() }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is purged" }),
        );
        await expect(
            client.secret.restore({ ...key, requestId: RequestId.create() }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is purged" }),
        );
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is in the trash" }),
        );
    },
);

test.each(TEST_DIALECTS)(
    "refuse deleting a secret while a binding targets it, as a bound resource waits on %s",
    async (dialect) => {
        // bind a secret to an installation's declaration
        await using fixture = await VaultFixture.open(dialect);
        const { client, database, spaceId, vaultId } = fixture;
        const created = await client.secret.create({
            spaceId,
            parentId: vaultId,
            name: "mail",
            requestId: RequestId.create(),
        });
        const now = Date.now();
        const installationId = identifier("installation").parse(`installation-${v7()}`);
        await database.insert(installation.table).values({
            id: installationId,
            scope: spaceId,
            packageId: VAULT.id,
            role: "application",
            alias: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        } as never);
        const bindingId = identifier("binding").parse(`binding-${v7()}`);
        await database.insert(binding.table).values({
            id: bindingId,
            scope: spaceId,
            installationId,
            packageId: VAULT.id,
            name: "token",
            target: created.id,
            version: null,
            state: {},
            createdAt: now,
            updatedAt: now,
        });
        const remove = () =>
            client.secret.delete({
                spaceId,
                id: created.id,
                revision: created.revision,
                requestId: RequestId.create(),
            });

        // refuse the deletion while bound, and accept it once the binding is gone
        await expect(remove()).rejects.toEqual(
            new ServiceError("CONFLICT", {
                defined: true,
                message: `${created.id} is in use: bound by ${installationId}`,
            }),
        );
        await database.delete(binding.table).where(eq(binding.table.id, bindingId));
        expect(await remove()).toEqual({});
    },
);
