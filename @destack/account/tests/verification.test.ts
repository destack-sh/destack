import { session } from "../src/object/authentication.ts";
import { Scope } from "@destack/sync";
import { account } from "../src/object/account.ts";
import { device } from "../src/object/device.ts";
import { user } from "../src/object/user.ts";
import { DirectoryStore } from "@destack/directory";
import { copyScope } from "@destack/access/test";
import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { identifier } from "@destack/schema";
import { PackageId } from "@destack/package";
import { v7 } from "uuid";
import {
    Restriction,
    Authorization,
    Authorizer,
    Relationship,
    accessRelationship,
    accessRole,
    accessRolePermission,
    principal,
} from "@destack/access";
import { TokenVerifier, TokenIssuer, Authentication } from "@destack/service/authentication";
import * as accountObject from "@destack/account/object";
import { serviceAccount } from "../src/object/service.ts";
import { Credential } from "../src/object/credential.ts";
import { personalAccessToken, serviceToken } from "../src/object/token.ts";
import { signingKey } from "../src/stack/authentication/key.ts";
import { AccountIntrospection, AccountToken, connect } from "../src/client/index.ts";
import { AccountFixture, id, identity } from "./fixture.ts";
import type {} from "@destack/package/import-meta";

/** Verify a space-scoped caller through authenticated HTTP without losing token restrictions. */
test("verify scoped callers and reject revoked credentials across services", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.opened.database;
    const authenticator = fixture.authenticator;
    const server = fixture.server;
    const now = Date.now();
    const record = { createdAt: now, updatedAt: now };
    const accountId = id("account");
    const userId = id("user");
    const regionId = id("region");
    const spaceId = id("space");
    const secretId = id("secret");
    const roleId = id("role");
    const softwareId = id("service-account");
    const softwareTokenId = id("token");
    const personalId = id("token");
    const hostKey = await Credential.create("service-token");
    const userKey = await Credential.create("personal-access-token");
    const audience = PackageId.parse(`package-${v7()}`);
    const permission = accountObject.account.permission("verify");
    const read = { packageId: audience, type: "version", name: "read" };

    // provision current identities, scoped grants and hashed credentials in real storage
    await database
        .insert(user.table)
        .values({ ...record, id: userId, name: "Owner", email: "owner@example.com" });
    await database.insert(account.table).values({
        ...record,
        id: accountId,
        handle: "owner",
        name: "Owner",
        defaultResidency: "eu",
        scope: userId,
    });
    const owner = principal.user.reference(Scope.universe.id, userId);
    const onAccount = accountObject.account.reference(userId, accountId);

    // place the space under the same account in a region cell
    await new DirectoryStore(database).place({
        id: spaceId,
        scope: accountId,
        cell: regionId,
        epoch: 1,
    });

    // root the account at its owner, who is also a member, and grant the receiving host verification authority
    await copyScope(database, owner);
    const accounts = new Authorizer(
        [accountObject.account.policy],
        [accountObject.account.mapping],
    );
    await new Authorization(accounts, database, () => ({
        subjects: [owner],
        now,
        attributes: {},
    })).create(onAccount, {
        relationships: [{ relation: "root", subject: owner }],
        owner: { ...onAccount, relation: "root" },
    });
    const membershipId = id("relationship");
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: membershipId,
                object: onAccount,
                relation: "member",
                subject: owner,
                createdAt: now,
                expiresAt: null,
            },
            accountId,
        ),
    );
    await database.insert(accessRole).values({
        ...record,
        id: roleId,
        scope: accountId,
        name: "verifier",
        description: "Verify Vault callers.",
    });
    await database
        .insert(accessRolePermission)
        .values({ id: id("role-permission"), roleId, scope: accountId, ...permission });
    await database
        .insert(serviceAccount.table)
        .values({ ...record, id: softwareId, scope: accountId, name: "vault-host" });
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: id("relationship"),
                object: accountObject.account.reference(userId, accountId),
                role: roleId,
                subject: serviceAccount.reference(accountId, softwareId),
                createdAt: now,
                expiresAt: null,
            },
            accountId,
        ),
    );
    const hostRestriction = { ...permission, scope: accountId };
    await database.insert(serviceToken.table).values({
        ...record,
        id: softwareTokenId,
        scope: accountId,
        parentId: softwareId,
        name: "host",
        digest: hostKey.digest,
        restrictions: [hostRestriction],
        expiresAt: now + 600000,
    });

    // restrict the presented credential independently from the receiving host
    const userRestriction = { ...read, scope: spaceId, objectId: secretId };
    await database.insert(personalAccessToken.table).values({
        ...record,
        id: personalId,
        scope: userId,
        name: "agent",
        digest: userKey.digest,
        restrictions: [userRestriction],
        expiresAt: now + 600000,
    });

    // route the host client and presented credential through the actual service transport
    let exchanges = 0;
    const userClient = connect({
        url: "http://localhost:3210",
        headers: { authorization: `Bearer ${userKey.secret}` },
        fetch: (request) => {
            exchanges++;

            return server.fetch(request);
        },
    });

    // exchange once, then verify many requests using cached public keys
    const retained = new AccountToken(userClient, { audience, spaceId });
    const concurrent = await Promise.all([retained.get(), retained.get(), retained.get()]);
    expect(concurrent).toEqual([concurrent[0], concurrent[0], concurrent[0]]);
    expect(exchanges).toBe(1);
    expect(await retained.headers()).toEqual({ authorization: `Bearer ${concurrent[0]}` });
    expect(exchanges).toBe(1);
    const issued = await userClient.authentication.exchange({ audience, spaceId });

    // describe a personal credential without treating its bearer as a separate person
    expect(identity(await userClient.authentication.current())).toEqual({
        subject: principal.user.reference(Scope.universe.id, userId),
        credential: { kind: "personal-access-token", id: personalId },
        profile: {
            subject: principal.user.reference(Scope.universe.id, userId),
            name: "Owner",
            handle: null,
        },
    });
    let keyReads = 0;
    const tokens = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: "http://localhost:3210",
        audience,
        keys: new URL("http://localhost:3210/auth/jwks"),
        fetch: async (url, options) => {
            keyReads++;

            return server.fetch(new Request(url, options));
        },
    });
    const signedRequest = new Request("https://vault.example/secret", {
        headers: { authorization: `Bearer ${issued.accessToken}` },
    });
    const signed = await tokens.authenticate(signedRequest, spaceId);
    expect(signed.claims.permissions).toEqual([{ ...read, scope: spaceId, objectId: secretId }]);
    await tokens.authenticate(signedRequest, spaceId);
    expect(keyReads).toBe(1);

    // refuse the signed token for another space and after its expiry
    await expect(tokens.authenticate(signedRequest, id("space"))).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid access token claims",
    });
    await expect(
        tokens.authenticate(signedRequest, spaceId, issued.expiresAt),
    ).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "caller authentication is expired or has a different audience or scope",
    });

    // rotate through Better Auth while retaining verification of unexpired old tokens
    await database.update(signingKey).set({ expiresAt: Date.now() - 1 });
    const rotated = await userClient.authentication.exchange({ audience, spaceId });
    const publicKeys = await (
        await server.fetch(new Request("http://localhost:3210/auth/jwks"))
    ).json();
    expect(publicKeys.keys).toHaveLength(2);
    const rotatedVerifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: "http://localhost:3210",
        audience,
        keys: publicKeys,
    });
    await rotatedVerifier.authenticate(signedRequest, spaceId);
    await rotatedVerifier.authenticate(
        new Request("https://vault.example/secret", {
            headers: { authorization: `Bearer ${rotated.accessToken}` },
        }),
        spaceId,
    );

    // inspect callers as the receiving host through its own token
    const client = connect({
        url: "http://localhost:3210",
        headers: { authorization: `Bearer ${hostKey.secret}` },
        fetch: (request) => server.fetch(request),
    });
    const verifier = new AccountIntrospection(client, accountId, audience);
    expect(identity(await client.authentication.current())).toEqual({
        subject: serviceAccount.reference(accountId, softwareId),
        credential: { kind: "service-token", id: softwareTokenId },
        profile: {
            subject: serviceAccount.reference(accountId, softwareId),
            name: "vault-host",
            handle: null,
        },
    });
    const rechecked = await verifier.authenticate(signedRequest, spaceId);
    expect(rechecked.claims.expiresAt).toBe(issued.expiresAt);
    expect(rechecked.claims.permissions).toEqual(signed.claims.permissions);

    // preserve delegated authority when the account service rechecks its primary credential
    const actor = serviceAccount.reference(accountId, softwareId);
    const issuer = new TokenIssuer({
        authority: { kind: "universe" },
        issuer: "http://localhost:3210",
        sign: async (payload) => (await authenticator.api.signJWT({ body: { payload } })).token,
    });
    const delegated = await issuer.issue(
        new Authentication({
            ...signed.claims,
            delegates: [{ subject: actor, authority: "lent" }],
        }),
    );
    const delegatedRequest = new Request("https://vault.example", {
        headers: { authorization: `Bearer ${delegated.accessToken}` },
    });
    const delegationCheck = await verifier.authenticate(delegatedRequest, spaceId);
    expect(delegationCheck.claims.delegates).toEqual([{ subject: actor, authority: "lent" }]);
    expect(delegationCheck.claims.expiresAt).toBe(issued.expiresAt);

    // retain the signed object restriction after the original credential receives broader access
    await database
        .update(personalAccessToken.table)
        .set({ restrictions: [{ ...read, scope: spaceId }] })
        .where(eq(personalAccessToken.table.id, personalId));
    expect((await verifier.authenticate(signedRequest, spaceId)).claims.permissions).toEqual(
        signed.claims.permissions,
    );
    await database
        .update(personalAccessToken.table)
        .set({ restrictions: [userRestriction] })
        .where(eq(personalAccessToken.table.id, personalId));

    // exchange a native session and block renewal when its enrolled device is revoked
    const deviceId = id("device");
    await database
        .insert(device.table)
        .values({ ...record, id: deviceId, scope: userId, name: "laptop" });
    const login = await (await authenticator.$context).internalAdapter.createSession(userId);
    await database
        .update(session.table)
        .set({ deviceId })
        .where(eq(session.table.id, identifier("session").parse(login.id)));
    const native = connect({
        url: "http://localhost:3210",
        headers: { authorization: `Bearer ${login.token}` },
        fetch: (request) => server.fetch(request),
    });
    const nativeToken = await native.authentication.exchange({ audience, spaceId });
    expect(identity(await native.authentication.current())).toEqual({
        subject: principal.user.reference(Scope.universe.id, userId),
        credential: { kind: "session", id: login.id },
        profile: {
            subject: principal.user.reference(Scope.universe.id, userId),
            name: "Owner",
            handle: null,
        },
    });
    const nativeRequest = new Request("https://vault.example", {
        headers: { authorization: `Bearer ${login.token}` },
    });
    expect((await verifier.authenticate(nativeRequest, spaceId)).claims.subject.id).toBe(userId);
    expect(
        (
            await rotatedVerifier.authenticate(
                new Request("https://vault.example", {
                    headers: { authorization: `Bearer ${nativeToken.accessToken}` },
                }),
                spaceId,
            )
        ).claims.subject.id,
    ).toBe(userId);
    await database
        .update(device.table)
        .set({ revokedAt: Date.now() })
        .where(eq(device.table.id, deviceId));
    await expect(native.authentication.current()).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "the person is not signed in",
    });
    await expect(native.authentication.exchange({ audience, spaceId })).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "the person is not signed in",
    });
    await expect(verifier.authenticate(nativeRequest, spaceId)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "the person is not signed in",
    });

    // service credentials use the same exchange and verifier without inventing a user session
    await database
        .update(serviceToken.table)
        .set({ restrictions: [hostRestriction, userRestriction] })
        .where(eq(serviceToken.table.id, softwareTokenId));
    const software = await client.authentication.exchange({ audience, spaceId });
    const softwareCaller = await rotatedVerifier.authenticate(
        new Request("https://vault.example", {
            headers: { authorization: `Bearer ${software.accessToken}` },
        }),
        spaceId,
    );
    expect(softwareCaller.claims.subject).toEqual(serviceAccount.reference(accountId, softwareId));
    expect(softwareCaller.claims.subjects).toEqual([softwareCaller.claims.subject]);
    const request = new Request("https://vault.example/secret", {
        headers: { authorization: `Bearer ${userKey.secret}` },
    });
    const caller = await verifier.authenticate(request, spaceId);
    expect(caller.context(audience, caller.claims.verifiedAt - 1000, spaceId).subject).toEqual(
        caller.claims.subject,
    );
    expect(() => caller.context(audience, caller.claims.verifiedAt - 5001, spaceId)).toThrow(
        "caller authentication is expired or has a different audience or scope",
    );
    expect(caller.claims.subjects).toEqual([
        principal.user.reference(Scope.universe.id, userId),
        { ...accountObject.account.reference(userId, accountId), relation: "member" },
        { ...accountObject.account.reference(userId, accountId), relation: "root" },
    ]);
    expect(caller.claims.permissions).toEqual([{ ...read, scope: spaceId, objectId: secretId }]);
    expect(
        Restriction.allows(
            read,
            { scope: spaceId, id: secretId },
            caller.context(audience, Date.now(), spaceId),
        ),
    ).toBe(true);
    expect(
        Restriction.allows(
            read,
            { scope: spaceId, id: "secret-other" },
            caller.context(audience, Date.now(), spaceId),
        ),
    ).toBe(false);

    // invalidate current membership without a caller cache
    await database.delete(accessRelationship).where(eq(accessRelationship.id, membershipId));
    expect((await verifier.authenticate(request, spaceId)).claims.subjects).toEqual([
        principal.user.reference(Scope.universe.id, userId),
        { ...onAccount, relation: "root" },
    ]);
    await database
        .update(personalAccessToken.table)
        .set({ revokedAt: Date.now() })
        .where(eq(personalAccessToken.table.id, personalId));

    // block renewal after revocation, and keep existing access tokens until their expiry
    await expect(userClient.authentication.exchange({ audience, spaceId })).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid personal access token",
    });
    await tokens.authenticate(signedRequest, spaceId);
    await expect(verifier.authenticate(signedRequest, spaceId)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid personal access token",
    });
    await expect(verifier.authenticate(request, spaceId)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid personal access token",
    });

    // revoke the receiving host independently of the credential it is inspecting
    await database
        .update(serviceToken.table)
        .set({ revokedAt: Date.now() })
        .where(eq(serviceToken.table.id, softwareTokenId));
    await expect(verifier.authenticate(request, spaceId)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "invalid service token",
    });
});
