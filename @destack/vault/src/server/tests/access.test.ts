import { expect, test } from "@destack/test";
import { createRequestId } from "@destack/service/request";
import { Caller } from "@destack/service/authentication";
import { and, eq } from "@destack/db";
import {
    secret,
    space,
    roleBinding,
    rolePermission,
    installation,
    deployment,
    serviceAccount,
    deploymentSecretBinding,
} from "@destack/model/regional";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { connect } from "../../secret/client.ts";
import { AuditOutbox } from "@destack/audit/outbox";
import { VaultFixture } from "./fixture.ts";
import { vaultPackage } from "../../audit/index.ts";

/** Authorize regional space secrets through an exact direct user grant. */
test("authorize direct user grants independently of account membership", async () => {
    await using fixture = await VaultFixture.open();
    const { database, client, context } = fixture;
    const { key } = await fixture.createSecret();
    const authority = "https://authority.test";
    const subject = { kind: "user" as const, authority, id: fixture.userId };

    // grant the user access without requiring membership in the space's account
    await database
        .update(roleBinding)
        .set({
            accountId: null,
            accountMembershipId: null,
            userAuthority: authority,
            userId: subject.id,
        })
        .where(eq(roleBinding.roleId, fixture.roleId));
    const authentication = {
        ...context.caller.authentication,
        subject,
        subjects: [subject],
        memberships: [],
    };
    context.caller = new Caller(authentication);
    expect((await client.version.read(key)).value).toEqual({
        encoding: "text",
        value: "credential",
    });

    // reject the same identifier authenticated by another authority
    const otherSubject = { ...subject, authority: "https://other-authority.test" };
    context.caller = new Caller({
        ...authentication,
        subject: otherSubject,
        subjects: [otherSubject],
    });
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
});

/** Preserve authorized administration while blocking suspended-space plaintext. */
test("deny plaintext while preserving suspended space administration", async () => {
    await using fixture = await VaultFixture.open();
    const { database, client, spaceId } = fixture;
    const { first, key } = await fixture.createSecret();
    const metadata = await client.secret.get(key);
    const plaintext = await client.version.read(key);

    // suspend the space without deleting its resources or granting new permissions
    await database.update(space).set({ status: "suspended" }).where(eq(space.id, spaceId));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    expect(await client.secret.get(key)).toEqual(metadata);

    // retain ordinary permission checks for administrative access while suspended
    await fixture.allow(false);
    await expect(client.secret.get({ spaceId, secretId: first.id })).rejects.toMatchObject({
        code: "FORBIDDEN",
    });
    await fixture.allow(true);

    // restore the same secret and binding without replacing their identities
    await database.update(space).set({ status: "enabled" }).where(eq(space.id, spaceId));
    const restored = await client.version.read(key);
    expect(restored).toEqual(plaintext);
});

/** Evaluate current scope, expiry and delegated authority before returning plaintext. */
test("restrict secret reads by object grants and delegated authority", async () => {
    await using fixture = await VaultFixture.open();
    const { client, database, context, spaceId } = fixture;
    const { first, key } = await fixture.createSecret();
    const other = await client.secret.create({
        spaceId,
        vaultId: fixture.vaultId,
        name: "other",
        requestId: createRequestId(),
    });
    const readPermission = and(
        eq(rolePermission.roleId, fixture.roleId),
        eq(rolePermission.type, "version"),
        eq(rolePermission.name, "read"),
    );

    // restrict an existing role to one exact secret and check the persisted restriction
    await database.update(rolePermission).set({ objectId: other.id }).where(readPermission);
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database.update(rolePermission).set({ objectId: first.id }).where(readPermission);
    expect((await client.version.read(key)).value).toEqual({
        encoding: "text",
        value: "credential",
    });

    // intersect authority-issued credential restrictions with an otherwise valid regional grant
    const unrestricted = context.caller.authentication;
    for (const permissions of [
        [],
        [
            {
                packageId: vaultPackage.id,
                type: "version",
                name: "read",
                scope: spaceId,
                objectId: other.id,
            },
        ],
    ]) {
        context.caller = new Caller({ ...unrestricted, permissions });
        await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    }
    context.caller = new Caller(unrestricted);

    // reject an expired binding and a membership asserted under another account
    const binding = await database
        .select()
        .from(roleBinding)
        .where(eq(roleBinding.roleId, fixture.roleId))
        .get();
    await database
        .update(roleBinding)
        .set({ expiresAt: binding!.createdAt + 1 })
        .where(eq(roleBinding.id, binding!.id));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database
        .update(roleBinding)
        .set({ expiresAt: null })
        .where(eq(roleBinding.id, binding!.id));
    const memberships = context.caller.authentication.memberships!;
    context.caller = new Caller({
        ...context.caller.authentication,
        memberships: [
            { ...memberships[0], accountId: identifier("account").parse(`account-${v7()}`) },
        ],
    });
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    context.caller = new Caller({ ...context.caller.authentication, memberships });

    // require both a current actor grant and an exact unexpired delegation
    const actor = {
        kind: "service-account" as const,
        authority: memberships[0].accountId,
        id: identifier("service-account").parse(`service-account-${v7()}`),
    };
    const actorBindingId = identifier("role-binding").parse(`role-binding-${v7()}`);
    const now = Date.now();
    await database.insert(roleBinding).values({
        id: actorBindingId,
        roleId: fixture.roleId,
        accountId: memberships[0].accountId,
        spaceId,
        accountServiceAccountId: actor.id,
        createdAt: now,
        updatedAt: now,
    });
    context.caller = new Caller({
        ...context.caller.authentication,
        subject: memberships[0].subject,
        actor,
        delegations: [
            {
                id: "credential-refresh",
                subject: memberships[0].subject,
                actor,
                permissions: [
                    {
                        packageId: vaultPackage.id,
                        type: "version",
                        name: "read",
                        scope: spaceId,
                        objectId: first.id,
                    },
                ],
                createdAt: now,
                expiresAt: now + 60000,
                revokedAt: null,
            },
        ],
    });
    expect((await client.version.read(key)).value).toEqual({
        encoding: "text",
        value: "credential",
    });
    await expect(client.secret.get(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await fixture.signCaller();
    expect((await fixture.client.version.read(key)).value).toEqual({
        encoding: "text",
        value: "credential",
    });
    await expect(fixture.client.secret.get(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database
        .update(roleBinding)
        .set({ revokedAt: Date.now() })
        .where(eq(roleBinding.id, actorBindingId));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await expect(fixture.client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database
        .update(roleBinding)
        .set({ revokedAt: null })
        .where(eq(roleBinding.id, actorBindingId));
    context.caller = new Caller({
        ...context.caller.authentication,
        delegations: [{ ...context.caller.authentication.delegations![0], revokedAt: Date.now() }],
    });
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
});

/** Bind software authority to active deployments and exact or current secret versions. */
test("restrict workload reads to live deployment secret bindings", async () => {
    await using fixture = await VaultFixture.open();
    const { client, database, context, spaceId } = fixture;
    const { first, key, written } = await fixture.createSecret();
    await client.version.write({
        ...key,
        revision: written.secret.revision,
        requestId: createRequestId(),
        value: { encoding: "text", value: "second" },
        promote: true,
    });

    // prepare unrelated deletion work that requires maintenance permission, without plaintext access
    const expired = await client.secret.create({
        spaceId,
        vaultId: fixture.vaultId,
        name: "expired",
        requestId: createRequestId(),
    });
    await client.secret.delete({
        spaceId,
        secretId: expired.id,
        revision: expired.revision,
        requestId: createRequestId(),
    });
    await database
        .update(secret)
        .set({ deleteAt: Date.now() - 1 })
        .where(eq(secret.id, expired.id));
    await database
        .update(rolePermission)
        .set({ objectId: spaceId })
        .where(
            and(
                eq(rolePermission.roleId, fixture.roleId),
                eq(rolePermission.type, "secret"),
                eq(rolePermission.name, "purge"),
            ),
        );

    // provision a running workload and grant its identity the same regional role
    const now = Date.now();
    const installationId = identifier("installation").parse(`installation-${v7()}`);
    const serviceAccountId = identifier("service-account").parse(`service-account-${v7()}`);
    const deploymentId = identifier("deployment").parse(`deployment-${v7()}`);
    await database.insert(installation).values({
        id: installationId,
        spaceId,
        packageId: vaultPackage.id,
        version: vaultPackage.version,
        alias: "consumer",
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(serviceAccount).values({
        id: serviceAccountId,
        spaceId,
        installationId,
        name: "consumer",
        workload: "main",
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(deployment).values({
        id: deploymentId,
        spaceId,
        installationId,
        generation: 1,
        packageId: vaultPackage.id,
        version: vaultPackage.version,
        manifest: "fixture",
        output: "main",
        workload: "main",
        runtime: "bun",
        serviceAccountId,
        description: {
            entrypoint: ".",
            export: "workload",
            services: [],
            schedules: [],
            resources: [],
            secrets: [{ packageId: vaultPackage.id, name: "credential" }],
            connections: [],
            compute: {},
        },
        policies: { packages: [], network: [] },
        status: "active",
        activatedAt: now,
        createdAt: now,
        updatedAt: now,
    });
    await database.insert(roleBinding).values({
        id: identifier("role-binding").parse(`role-binding-${v7()}`),
        roleId: fixture.roleId,
        spaceId,
        serviceAccountId,
        createdAt: now,
        updatedAt: now,
    });
    context.caller = new Caller({
        ...context.caller.authentication,
        subject: { kind: "service-account", authority: spaceId, id: serviceAccountId },
        subjects: [{ kind: "service-account", authority: spaceId, id: serviceAccountId }],
        attributes: {},
        memberships: [],
    });

    // neither a broad role nor a deployment alone permits an unbound secret read
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    context.caller = new Caller({
        ...context.caller.authentication,
        deployments: [{ subject: context.caller.authentication.subject, id: deploymentId }],
    });
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await fixture.signCaller(spaceId);
    expect(await client.secret.purge({ spaceId, limit: 100 })).toBe(1);
    await database.insert(deploymentSecretBinding).values({
        spaceId,
        deploymentId,
        packageId: vaultPackage.id,
        name: "credential",
        secretId: first.id,
        version: 1,
    });
    expect((await fixture.client.version.read({ ...key, version: 1 })).value).toEqual({
        encoding: "text",
        value: "credential",
    });
    expect((await client.version.read({ ...key, version: 1 })).value).toEqual({
        encoding: "text",
        value: "credential",
    });
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });

    // current bindings follow promotion but never expose an arbitrary old version
    await database
        .update(deploymentSecretBinding)
        .set({ version: null })
        .where(eq(deploymentSecretBinding.deploymentId, deploymentId));
    expect((await client.version.read(key)).value).toEqual({ encoding: "text", value: "second" });
    await expect(client.version.read({ ...key, version: 1 })).rejects.toMatchObject({
        code: "FORBIDDEN",
    });

    // suspension, retirement and identity revocation take effect on the next read
    await database
        .update(installation)
        .set({ status: "suspended" })
        .where(eq(installation.id, installationId));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database
        .update(installation)
        .set({ status: "enabled" })
        .where(eq(installation.id, installationId));
    await database
        .update(deployment)
        .set({ status: "retired", retiredAt: Date.now() })
        .where(eq(deployment.id, deploymentId));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await database
        .update(deployment)
        .set({ status: "active", retiredAt: null })
        .where(eq(deployment.id, deploymentId));
    await database
        .update(serviceAccount)
        .set({ revokedAt: Date.now() })
        .where(eq(serviceAccount.id, serviceAccountId));
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await expect(fixture.client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
});

test("audit denied calls and recheck authority on retries", async () => {
    await using fixture = await VaultFixture.open();
    const { client, database } = fixture;
    const { key, request, written } = await fixture.createSecret();
    const anonymous = connect({
        url: "http://vault.test",
        fetch: (request) => fixture.server.fetch(request),
    });
    await expect(anonymous.secret.get(key)).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    const rejected = await new AuditOutbox(database).read(1000);
    expect(
        rejected
            .filter((event) => event.context.actor.type === "anonymous")
            .map((event) => event.result),
    ).toEqual([
        { stage: "attempt" },
        { stage: "result", outcome: "denied", errorCode: "UNAUTHORIZED" },
    ]);

    // deny plaintext separately and recheck authority before retry responses
    await fixture.allowRead(false);
    expect(await client.secret.get(key)).toEqual(written.secret);
    await expect(client.version.read(key)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await fixture.allow(false);
    await expect(client.version.write(request)).rejects.toMatchObject({ code: "FORBIDDEN" });
    await fixture.allow(true);
    await fixture.allowRead(true);
});
