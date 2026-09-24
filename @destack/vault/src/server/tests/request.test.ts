import { writeVersion } from "../../secret/version.ts";
import { PackageId } from "@destack/package";
import { expect, test } from "@destack/test";
import { createRequestId, REQUEST_LIFETIME_MS } from "@destack/service/request";
import { IdempotencyStore } from "@destack/service/database";
import { Caller } from "@destack/service/authentication";
import { and, eq } from "@destack/db";
import { secretVersion } from "@destack/model/regional";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { ResourceContext } from "@destack/resource/context";
import { defineSecret } from "../../declare/index.ts";
import { BoundSecret } from "../../secret/client.ts";
import { AuditRecorder } from "@destack/audit";
import { VaultFixture } from "./fixture.ts";
import { vaultRequest } from "../../stack/index.ts";
import { vaultPackage } from "../../audit/index.ts";

/** Reject stale identities and expired mutations even after their journal entries are removed. */
test("enforce caller freshness and bounded mutation replay", async () => {
    await using fixture = await VaultFixture.open();
    const { client, context, database, spaceId, vaultId } = fixture;
    const authentication = context.caller.authentication;
    const input = { spaceId, vaultId, name: "retained", requestId: createRequestId() };
    const created = await client.secret.create(input);
    expect(await client.secret.create(input)).toEqual(created);

    // reject another audience and expired verification before replaying a successful response
    for (const changed of [
        { audience: PackageId.parse("package-019f7480-0000-7000-8000-000000000001") },
        { verifiedAt: Date.now() - 60001 },
        { expiresAt: Date.now() - 1 },
    ]) {
        context.caller = new Caller({ ...authentication, ...changed });
        await expect(client.secret.create(input)).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    }
    context.caller = new Caller(authentication);

    // reject expired and future UUIDs without creating secrets or journal entries
    for (const msecs of [Date.now() - REQUEST_LIFETIME_MS - 1, Date.now() + 600000]) {
        await expect(
            client.secret.create({ ...input, requestId: v7({ msecs }) }),
        ).rejects.toMatchObject({ code: "PRECONDITION_FAILED" });
    }

    // remove an expired persisted result and retain the timestamp-based replay rejection
    const requestId = v7({ msecs: Date.now() - REQUEST_LIFETIME_MS - 1 });
    await database.insert(vaultRequest).values({
        caller: context.caller.id,
        scope: spaceId,
        procedure: "secret.create",
        requestId,
        digest: "expired",
        response: { value: null },
        createdAt: 1,
        expiresAt: 2,
    });
    const journal = new IdempotencyStore(vaultRequest);
    expect(await journal.prune(database, 100)).toBe(1);
    await expect(client.secret.create({ ...input, requestId })).rejects.toMatchObject({
        code: "PRECONDITION_FAILED",
    });

    // preserve a JSON null response through the same transactional protocol used by services
    const request = {
        caller: context.caller.id,
        scope: spaceId,
        procedure: "null",
        requestId: createRequestId(),
    };
    await database.transaction(async (transaction) => {
        expect(
            await journal.begin(
                transaction,
                request,
                { digest: "same" },
                (value) => value === "same",
            ),
        ).toEqual({ kind: "new" });
        await journal.complete(transaction, request, null);
    });
    await database.transaction(async (transaction) => {
        expect(
            await journal.begin(
                transaction,
                request,
                { digest: "same" },
                (value) => value === "same",
            ),
        ).toEqual({ kind: "replay", value: null });
    });
    await expect(
        journal.begin(database, request, { digest: "same" }, () => true),
    ).rejects.toMatchObject({ code: "INTERNAL_SERVER_ERROR" });
});

/** Serialize competing updates and roll back mutations when their audit cannot commit. */
test("serialize competing writes and roll back failed audit recording", async () => {
    const fixture = await VaultFixture.open();
    const { client, database, spaceId, vaultId } = fixture;
    try {
        const created = await client.secret.create({
            spaceId,
            vaultId,
            name: "concurrent",
            requestId: createRequestId(),
        });
        const key = { spaceId, secretId: created.id };
        const write = {
            ...key,
            revision: created.revision,
            requestId: createRequestId(),
            value: { encoding: "text" as const, value: "first" },
        };

        // concurrent delivery of one request commits exactly one version
        const repeated = await Promise.all([
            client.version.write(write),
            client.version.write(write),
        ]);
        expect(repeated[0]).toEqual(repeated[1]);
        const declaration = defineSecret({ name: "github-token" });
        const resources = new ResourceContext().bind(
            declaration,
            new BoundSecret(client, { space: spaceId, secret: created.id }),
        );
        expect((await declaration.get(resources).read()).value).toEqual(write.value);
        expect((await client.version.list(key)).items.map((version) => version.version)).toEqual([
            1,
        ]);

        // concurrent distinct writes cannot both satisfy the same revision
        const revision = repeated[0].secret.revision;
        const attempts = await Promise.allSettled([
            client.version.write({
                ...write,
                revision,
                requestId: createRequestId(),
                value: { encoding: "text", value: "second" },
            }),
            client.version.write({
                ...write,
                revision,
                requestId: createRequestId(),
                value: { encoding: "text", value: "third" },
            }),
        ]);
        expect(attempts.filter((attempt) => attempt.status === "fulfilled")).toHaveLength(1);
        expect(
            attempts
                .filter((attempt) => attempt.status === "rejected")
                .map((attempt) => attempt.reason.code),
        ).toEqual(["PRECONDITION_FAILED"]);
        expect((await client.version.list(key)).items.map((version) => version.version)).toEqual([
            1, 2,
        ]);
        const firstPage = await client.version.list({ ...key, limit: 1 });
        expect(firstPage.items.map((version) => version.version)).toEqual([1]);
        expect(firstPage.cursor).not.toBeNull();
        const secondPage = await client.version.list({
            ...key,
            limit: 1,
            cursor: firstPage.cursor!,
        });
        expect(secondPage.items.map((version) => version.version)).toEqual([2]);
        expect(secondPage.cursor).toBeNull();

        // bind reads to the full space identity and enforce version availability
        const otherSpace = identifier("space").parse(`space-${v7()}`);
        await expect(client.version.read({ ...key, spaceId: otherSpace })).rejects.toMatchObject({
            code: "NOT_FOUND",
        });
        const current = await client.secret.get(key);
        const disabled = await client.version.disable({
            ...key,
            version: 1,
            revision: current.revision,
            requestId: createRequestId(),
        });
        expect(disabled.disabledAt).not.toBeNull();
        await expect(client.version.read({ ...key, version: 1 })).rejects.toMatchObject({
            code: "FORBIDDEN",
        });
        const afterDisable = await client.secret.get(key);
        await client.version.enable({
            ...key,
            version: 1,
            revision: afterDisable.revision,
            requestId: createRequestId(),
        });
        expect((await client.version.read({ ...key, version: 1 })).value).toEqual(write.value);

        // expire an existing version without timers or a test-specific clock in the service
        await database
            .update(secretVersion)
            .set({ createdAt: Date.now() - 2000, expiresAt: Date.now() - 1000 })
            .where(and(eq(secretVersion.secretId, key.secretId), eq(secretVersion.version, 1)));
        await expect(client.version.read({ ...key, version: 1 })).rejects.toMatchObject({
            code: "FORBIDDEN",
        });

        // reject a failed audit without advancing the secret or retaining a retry result
        const before = await client.secret.get(key);
        const audit = new AuditRecorder(
            {
                actor: { type: "user", authority: "global", id: fixture.userId },
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
        const rejected = { ...write, revision: before.revision, requestId: createRequestId() };
        await expect(
            writeVersion(fixture.vault, rejected, { ...fixture.context, audit }),
        ).rejects.toThrow("audit unavailable");
        expect(await client.secret.get(key)).toEqual(before);
        expect((await client.version.write(rejected)).version.version).toBe(3);
    } finally {
        await fixture.close();
    }
});
