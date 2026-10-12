import { Caller, principal } from "@destack/access";
import { TEST_DIALECTS } from "@destack/db/test";
import { testCallKey } from "@destack/service/test";
import { Journal } from "@destack/audit/server";
import { eq, isNotNull } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { expect, refusal, single, test } from "@destack/test";
import { Clock, Duration, present, schema } from "@destack/schema";
import type { ObjectCall } from "@destack/object";
import { SecretReading } from "../secret/index.ts";
import { binding, installation } from "@destack/stack/object";
import { Scope } from "@destack/sync";
import { v7 } from "uuid";
import { SECRET_RECOVERY, secret, secretVersion, vault } from "../object/index.ts";
import { MACHINE, OTHER_MACHINE, VAULT, VaultFixture } from "../test/index.ts";

/** The recovery window the fixture's machine gives deleted secrets, in milliseconds. */
const RECOVERY_MILLISECONDS = Duration.milliseconds(SECRET_RECOVERY);

/** How long the purge controller may take to purge an expired secret. */
const PURGE_WAIT_MILLISECONDS = 5000;

test.each(TEST_DIALECTS)(
    "roundtrip bounded text and binary values without committing rejected writes on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client, spaceId, vaultId } = fixture;
        const created = await client.secret.create({
            scope: spaceId,
            parentId: vaultId,
            name: "binary",
            requestId: RequestId.create(Date.now()),
        });
        const key = { scope: spaceId, id: created.id };
        const value = {
            encoding: "base64" as const,
            value: Uint8Array.from({ length: 65536 }, (_, index) => index % 256).toBase64(),
        };
        const written = await client.version.create({
            scope: spaceId,
            parentId: created.id,
            requestId: RequestId.create(Date.now()),
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
                    scope: spaceId,
                    parentId: created.id,
                    requestId: RequestId.create(Date.now()),
                    value: rejected,
                }),
            ).rejects.toEqual(
                new ServiceError("PAYLOAD_TOO_LARGE", {
                    message: "secret value exceeds 65536 bytes",
                }),
            );
        }
        expect(
            await refusal(
                client.version.create({
                    scope: spaceId,
                    parentId: created.id,
                    requestId: RequestId.create(Date.now()),
                    value: { encoding: "base64", value: "!" },
                }),
            ),
        ).toEqual(["BAD_REQUEST", "invalid input: value.value: invalid base64-encoded string"]);
        const versions = await client.version.list({ scope: spaceId });
        expect(versions.items).toEqual([written]);

        // accept the exact UTF-8 limit and keep the earlier binary version readable
        const text = { encoding: "text" as const, value: "é".repeat(32768) };
        await client.version.create({
            scope: spaceId,
            parentId: created.id,
            requestId: RequestId.create(Date.now()),
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
            scope: spaceId,
            parentId: vaultId,
            requestId: RequestId.create(Date.now()),
            name: "github",
        });
        const key = { scope: spaceId, id: created.id };
        expect(await client.secret.get(key)).toEqual(created);

        // keep one version across retries of the same request
        const request = {
            scope: spaceId,
            parentId: created.id,
            requestId: RequestId.create(Date.now()),
            value: { encoding: "text" as const, value: "oauth-refresh-token" },
            stage: "current" as const,
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
        await expect(client.version.create({ ...request, stage: "pending" })).rejects.toEqual(
            new ServiceError("CONFLICT", {
                defined: true,
                message: "request identifier has already been used",
            }),
        );
        expect(await client.secret.read(key)).toEqual({ version: 1, value: request.value });

        // stage a second version before selecting it as current
        const staged = await client.version.create({
            scope: spaceId,
            parentId: created.id,
            requestId: RequestId.create(Date.now()),
            value: { encoding: "base64", value: "AAECAw==" },
            stage: "pending" as const,
        });
        expect(await client.secret.read(key)).toEqual({ version: 1, value: request.value });
        const promoted = await client.secret.promote({
            ...key,
            requestId: RequestId.create(Date.now()),
            version: staged.number,
        });
        expect(promoted).toEqual({ ...(await client.secret.get(key)), currentVersion: 2 });
        expect(await client.secret.read(key)).toEqual({
            version: 2,
            value: { encoding: "base64", value: "AAECAw==" },
        });

        // keep explicit disabling across deletion and restoration
        await client.secret.disable({ ...key, requestId: RequestId.create(Date.now()) });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "secret is unavailable" }),
        );
        const disabled = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            requestId: RequestId.create(Date.now()),
            revision: disabled.revision,
        });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is in the trash" }),
        );
        await client.secret.restore({ ...key, requestId: RequestId.create(Date.now()) });
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "secret is unavailable" }),
        );
        await client.secret.enable({ ...key, requestId: RequestId.create(Date.now()) });
        expect(await client.secret.read(key)).toEqual({
            version: 2,
            value: { encoding: "base64", value: "AAECAw==" },
        });

        // keep a destroyed version's record and refuse its value
        const time = fixture.clock.freeze();
        const destroyed = await client.version.destroy({
            scope: spaceId,
            id: written.id,
            requestId: RequestId.create(Date.now()),
        });
        expect(destroyed).toEqual({
            ...written,
            destroyedAt: time,
            keyId: null,
            revision: 2,
            updatedAt: time,
        });
        await expect(client.secret.read({ ...key, version: 1 })).rejects.toEqual(
            new ServiceError("FORBIDDEN", {
                defined: true,
                message: "secret version is unavailable",
            }),
        );
        await expect(
            client.version.disable({
                scope: spaceId,
                id: written.id,
                requestId: RequestId.create(Date.now()),
            }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret version is destroyed" }),
        );
        const ciphertext = await database
            .select({ version: secretVersion.table.number })
            .from(secretVersion.table)
            .where(isNotNull(secretVersion.table.ciphertext));
        expect(ciphertext.map((row) => row.version)).toEqual([2]);

        // record every change, disclosure and refusal without plaintext
        const calls = await new Journal(database, testCallKey, { clock: Clock.system }).read({
            limit: 1000,
        });
        const actions = calls.filter(
            (call) =>
                call.execution.context.package.id === VAULT.id &&
                call.execution.category !== "denial",
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

        // record the version each read disclosed
        const reads = actions.filter((call) => call.method === "secret.read");
        expect(reads.map((call) => call.execution.details)).toEqual([
            { version: 1 },
            { version: 1 },
            { version: 2 },
            {},
            { version: 2 },
        ]);

        // record each refused read once
        const denials = calls.filter((call) => call.execution.category === "denial");
        expect(denials.map((call) => [call.execution.target, call.execution.outcome])).toEqual(
            ["secret is unavailable", "secret is unavailable", "secret version is unavailable"].map(
                (message) => [
                    { type: "procedure", id: "secret.read" },
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
                requestId: RequestId.create(Date.now()),
                expiresAt: fixture.clock.now() - 1000,
            }),
        ).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "secret expiry must be in the future",
            }),
        );

        // read a version before its expiry
        const expiring = await client.version.create({
            ...request,
            requestId: RequestId.create(Date.now()),
            expiresAt: fixture.clock.now() + 60_000,
        });
        expect(await client.secret.read(key)).toEqual({ version: 2, value: request.value });

        // refuse reading the version once its expiry passed
        await database
            .update(secretVersion.table)
            .set({ createdAt: fixture.clock.now() - 2000, expiresAt: fixture.clock.now() - 1000 })
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
            client.secret.purge({ ...key, requestId: RequestId.create(Date.now()) }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is not deleted" }),
        );
        const row = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            revision: row.revision,
            requestId: RequestId.create(Date.now()),
        });

        // purge another deleted secret within its window
        const other = await client.secret.create({
            scope: spaceId,
            parentId: vaultId,
            name: "other",
            requestId: RequestId.create(Date.now()),
        });
        const otherKey = { scope: spaceId, id: other.id };
        await client.secret.delete({
            ...otherKey,
            revision: other.revision,
            requestId: RequestId.create(Date.now()),
        });
        const purgeTime = fixture.clock.freeze();
        expect(
            await client.secret.purge({ ...otherKey, requestId: RequestId.create(Date.now()) }),
        ).toEqual({});
        expect((await client.secret.get(otherKey)).purgedAt).toBe(purgeTime);

        // destroy every value after the window ends and keep the secret and its versions as records
        await database
            .update(secret.table)
            .set({ deletionRequestedAt: purgeTime - RECOVERY_MILLISECONDS })
            .where(eq(secret.table.id, first.id));
        const isPurged = await database.log.until(async () => {
            const { purgedAt } = single(
                await database
                    .select({ purgedAt: secret.table.purgedAt })
                    .from(secret.table)
                    .where(eq(secret.table.id, first.id)),
            );

            return purgedAt !== null;
        }, AbortSignal.timeout(PURGE_WAIT_MILLISECONDS));
        expect(isPurged).toBe(true);
        const version = single(
            await database
                .select()
                .from(secretVersion.table)
                .where(eq(secretVersion.table.id, written.id)),
        );
        expect([version.destroyedAt, version.ciphertext]).toEqual([purgeTime, null]);
        const purged = single(
            await database.select().from(secret.table).where(eq(secret.table.id, first.id)),
        );
        expect(purged.purgedAt).toBe(purgeTime);

        // refuse purging, restoring and reading a purged secret
        await expect(
            client.secret.purge({ ...key, requestId: RequestId.create(Date.now()) }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", { defined: true, message: "secret is purged" }),
        );
        await expect(
            client.secret.restore({ ...key, requestId: RequestId.create(Date.now()) }),
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
            scope: spaceId,
            parentId: vaultId,
            name: "mail",
            requestId: RequestId.create(Date.now()),
        });
        const now = fixture.clock.now();
        const installationId = schema.identifier("installation").parse(`installation-${v7()}`);
        await database.insert(installation.table).values({
            id: installationId,
            scope: spaceId,
            packageId: VAULT.id,
            role: "application",
            alias: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        });
        const bindingId = schema.identifier("binding").parse(`binding-${v7()}`);
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
                scope: spaceId,
                id: created.id,
                revision: created.revision,
                requestId: RequestId.create(Date.now()),
            });

        // refuse the deletion while bound
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

test("put a consumer's secret as one live version through its vault, read it as the consumer, conceal it from another machine, and remove it", async () => {
    // put and remove the secret through the vault, bound to a machine
    await using fixture = await VaultFixture.open("sqlite");
    const objects = fixture.objects();
    const machine = principal.machine.reference(Scope.universe.id, MACHINE);
    const stranger = principal.machine.reference(Scope.universe.id, OTHER_MACHINE);
    const target = vault.target({ scope: fixture.spaceId, id: fixture.vaultId });
    const id = schema.identifier("secret").parse(secret.generateId());
    const write = (call: ObjectCall) =>
        objects.mutate(Caller.of(objects.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [call],
        });
    const put = (value: string) =>
        write(
            vault.call("put", {
                ...target,
                secret: id,
                value: { encoding: "text", value },
                consumer: machine,
            }),
        );
    const read = async (reader: typeof machine) => {
        const [row] = await fixture.database
            .select()
            .from(secret.table)
            .where(eq(secret.table.id, id));
        const version = present(row?.currentVersion, "the secret's current version");
        const call = secret.call("read", {
            ...secret.target({ scope: fixture.spaceId, id }),
            version,
        });

        return SecretReading.parse(await objects.query(Caller.of(reader, Date.now()), call)).value;
    };

    // put twice and read the live value as the machine, refusing another machine
    await put("first");
    await put("second");
    const live = await read(machine);
    const versions = await fixture.database.select().from(secretVersion.table);
    const concealed = await refusal(read(stranger));

    // remove it with its values
    await write(vault.call("remove", { ...target, secret: id, consumer: machine }));
    const [removed] = await fixture.database
        .select()
        .from(secret.table)
        .where(eq(secret.table.id, id));

    expect({
        live,
        versions: versions.map((row) => [row.number, row.destroyedAt === null]),
        concealed,
        purged: removed?.purgedAt !== null,
    }).toEqual({
        live: { encoding: "text", value: "second" },
        versions: [
            [1, false],
            [2, true],
        ],
        concealed: ["NOT_FOUND", `no secret ${id}`],
        purged: true,
    });
});
