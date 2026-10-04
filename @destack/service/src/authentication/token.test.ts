import { expect, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { exportJWK, generateKeyPair, SignJWT, type JWTPayload } from "jose";
import { TokenVerifier } from "./token.ts";
import { TokenIssuer } from "./issuer.ts";
import { Authentication, INSTALLATION_KEY } from "./authentication.ts";
import { principal, Restriction, Caller } from "@destack/access";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** Verify signed identity and reject cross-service, cross-space and stale authority. */
test("verify scoped tokens and reject invalid claims and signatures", async () => {
    const keys = await generateKeyPair("ES256");
    const publicKey = { ...(await exportJWK(keys.publicKey)), kid: "current", alg: "ES256" };
    const issuer = "https://account.example";
    const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const spaceId = "space-019f7480-0000-7000-8000-000000000002";
    const subject: Subject = principal.user.reference("universe", "user-example");
    const issuedAt = Math.floor(Date.now() / 1000);
    const claims = {
        iss: issuer,
        aud: audience,
        sub: subject.id,
        iat: issuedAt,
        exp: issuedAt + 60,
        jti: "token-example",
        token_use: "access",
        caller: {
            spaceId,
            credential: { kind: "user", id: "session-example" },
            subject,
            subjects: [subject],
        },
    };
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    const signed = await new SignJWT(claims)
        .setProtectedHeader({ alg: "ES256", kid: "current" })
        .sign(keys.privateKey);
    const request = new Request("https://service.example", {
        headers: { authorization: `Bearer ${signed}` },
    });
    const caller = await verifier.authenticate(request, spaceId, issuedAt * 1000);
    expect(caller.context(audience, issuedAt * 1000, spaceId).subject).toEqual(subject);
    expect(() => caller.context(audience, issuedAt * 1000, "another-space")).toThrow(
        "caller authentication is expired or has a different audience or scope",
    );

    // accept an administrator impersonating a user and reject other impersonations
    const administrator: Subject = principal.user.reference("universe", "user-administrator");
    const impersonate = async (impersonator: Subject) =>
        verifier.authenticate(
            new Request(request, {
                headers: {
                    authorization: `Bearer ${await new SignJWT({
                        ...claims,
                        caller: {
                            ...claims.caller,
                            delegates: [{ subject: impersonator, authority: "full" }],
                        },
                    })
                        .setProtectedHeader({ alg: "ES256", kid: "current" })
                        .sign(keys.privateKey)}`,
                },
            }),
            spaceId,
            issuedAt * 1000,
        );
    const impersonated = (await impersonate(administrator)).context(
        audience,
        issuedAt * 1000,
        spaceId,
    );
    expect([impersonated.subject, impersonated.delegates]).toEqual([
        subject,
        [{ subject: administrator, authority: "full" }],
    ]);
    for (const impersonator of [
        subject,
        principal.installation.reference("universe", "installation-example"),
    ]) {
        await expect(impersonate(impersonator)).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    }

    // reject changed claims signed by a trusted key
    for (const changed of [
        { iss: "https://other.example" },
        { aud: "another-package" },
        { sub: "another-user" },
        { exp: issuedAt },
        { iat: issuedAt + 10, exp: issuedAt + 60 },
        { exp: issuedAt + 61 },
        { token_use: "identity" },
        { jti: "" },
        {
            caller: {
                ...claims.caller,
                spaceId: "space-019f7480-0000-7000-8000-000000000003",
            },
        },
    ]) {
        const token = await new SignJWT({ ...claims, ...changed })
            .setProtectedHeader({ alg: "ES256", kid: "current" })
            .sign(keys.privateKey);
        await expect(
            verifier.authenticate(
                new Request(request, { headers: { authorization: `Bearer ${token}` } }),
                spaceId,
                issuedAt * 1000,
            ),
        ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    }

    // reject a token signed by an untrusted key
    const attacker = await generateKeyPair("ES256");
    const forged = await new SignJWT(claims)
        .setProtectedHeader({ alg: "ES256", kid: "current" })
        .sign(attacker.privateKey);
    await expect(
        verifier.authenticate(
            new Request(request, { headers: { authorization: `Bearer ${forged}` } }),
            spaceId,
        ),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    await expect(
        verifier.authenticate(request, spaceId, (issuedAt + 60) * 1000),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // keep cached public keys during an outage
    let isAvailable = true;
    let reads = 0;
    const remote = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: new URL(`${issuer}/jwks`),
        fetch: async () => {
            reads++;

            return isAvailable
                ? Response.json({ keys: [publicKey] })
                : new Response(null, { status: 503 });
        },
    });
    await remote.authenticate(request, spaceId, issuedAt * 1000);
    isAvailable = false;
    await remote.authenticate(request, spaceId, (issuedAt + 30) * 1000);
    expect(reads).toBe(1);
    await expect(
        remote.authenticate(request, spaceId, (issuedAt + 60) * 1000),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // require discovery without cached public keys
    const disconnected = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: new URL(`${issuer}/jwks`),
        fetch: async () => new Response(null, { status: 503 }),
    });
    await expect(
        disconnected.authenticate(request, spaceId, issuedAt * 1000),
    ).rejects.toMatchObject({ code: "SERVICE_UNAVAILABLE" });

    // keep workload identity and delegates through signing and verification
    const actor: Subject = principal.installation.reference(spaceId, "software-example");
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000004");
    const read = { packageId: PackageId.parse(audience), type: "note", name: "read" };
    const selection = { ...read, scope: spaceId, objectId: "note-one" };
    const delegated = new Authentication({
        ...caller.claims,
        deployments: [{ subject: actor, id: deploymentId }],
        delegates: [{ subject: actor, authority: "lent" }],
        permissions: [selection],
        assurance: { level: 2, authenticatedAt: issuedAt * 1000 },
        contacts: [{ medium: "email", address: "alice@example.com" }],
    });
    const signing = {
        authority: { kind: "universe" as const },
        issuer,
        sign: async (payload: JWTPayload) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current" })
                .sign(keys.privateKey),
    };
    const authority = new TokenIssuer(signing);
    const issued = await authority.issue(delegated, issuedAt * 1000);
    const represented = await verifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${issued.accessToken}` },
        }),
        spaceId,
        issuedAt * 1000,
    );
    expect(represented.claims.deployments).toEqual([{ subject: actor, id: deploymentId }]);
    expect(represented.claims.delegates).toEqual([{ subject: actor, authority: "lent" }]);
    const access = represented.context(audience, issuedAt * 1000, spaceId);
    expect([access.assurance, access.contacts]).toEqual([
        { level: 2, authenticatedAt: issuedAt * 1000 },
        [{ medium: "email", address: "alice@example.com" }],
    ]);
    expect(Caller.delegation(access)).toEqual([
        { delegate: actor, delegator: subject, authority: "lent" },
    ]);
    expect(
        Restriction.allows({ ...read, name: "write" }, { scope: spaceId, id: "note-one" }, access),
    ).toBe(false);

    // keep a deployment for each workload in a delegation chain
    const secondActor = { ...actor, id: "second-software" };
    const secondDeployment = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000005");
    const chain = new Authentication({
        ...delegated.claims,
        deployments: [
            { subject: actor, id: deploymentId },
            { subject: secondActor, id: secondDeployment },
        ],
        delegates: [
            { subject: actor, authority: "lent" },
            { subject: secondActor, authority: "lent" },
        ],
    });
    const chained = await authority.issue(chain, issuedAt * 1000);
    const chainedCaller = await verifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${chained.accessToken}` },
        }),
        spaceId,
        issuedAt * 1000,
    );
    expect(chainedCaller.claims.deployments).toEqual(chain.claims.deployments);
    expect(Caller.delegation(chainedCaller.context(audience, issuedAt * 1000, spaceId))).toEqual([
        { delegate: actor, delegator: subject, authority: "lent" },
        { delegate: secondActor, delegator: actor, authority: "lent" },
    ]);
    await expect(
        authority.issue(
            new Authentication({
                ...chain.claims,
                deployments: [{ subject: secondActor, id: secondDeployment }],
            }),
            issuedAt * 1000,
        ),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // reject malformed chains signed outside TokenIssuer
    for (const delegates of [
        [],
        [actor, actor],
        [subject, actor],
        [{ ...actor, kind: "user" as const }],
        [secondActor],
    ]) {
        const token = await signing.sign({
            ...claims,
            exp: issuedAt + 30,
            caller: {
                ...claims.caller,
                actor,
                deployments: [{ subject: actor, id: deploymentId }],
                delegates,
            },
        });
        await expect(
            verifier.authenticate(
                new Request(request, {
                    headers: { authorization: `Bearer ${token}` },
                }),
                spaceId,
                issuedAt * 1000,
            ),
        ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    }

    // reject space authorities delegating for global users
    const local = new TokenIssuer({ ...signing, authority: { kind: "space", spaceId } });
    await expect(local.issue(delegated, issuedAt * 1000)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
    });
    const localVerifier = new TokenVerifier({
        authority: { kind: "space", spaceId },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    await expect(
        localVerifier.authenticate(request, spaceId, issuedAt * 1000),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // accept only the space's own deployed software
    const workload = new Authentication({
        ...caller.claims,
        credential: { kind: "workload", id: "credential-example" },
        subject: actor,
        subjects: [actor],
        deployments: [{ subject: actor, id: deploymentId }],
    });
    const workloadToken = await local.issue(workload, issuedAt * 1000);
    const received = await localVerifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${workloadToken.accessToken}` },
        }),
        spaceId,
        issuedAt * 1000,
    );
    expect(received.claims.subject).toEqual(actor);
    expect(received.claims.deployments).toEqual([{ subject: actor, id: deploymentId }]);
    const { deployments: _deployments, ...withoutDeployments } = workload.claims;
    await expect(
        local.issue(new Authentication(withoutDeployments), issuedAt * 1000),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // accept the installation its own key authenticated without a deployment, and no deployment for it
    const keyed = new Authentication({
        ...withoutDeployments,
        credential: { kind: INSTALLATION_KEY, id: "installation-key-example" },
    });
    const keyedToken = await local.issue(keyed, issuedAt * 1000);
    const keyedCaller = await localVerifier.authenticate(
        new Request(request, { headers: { authorization: `Bearer ${keyedToken.accessToken}` } }),
        spaceId,
        issuedAt * 1000,
    );
    expect([keyedCaller.claims.subject, keyedCaller.claims.deployments]).toEqual([
        actor,
        undefined,
    ]);
    await expect(
        local.issue(
            new Authentication({
                ...keyed.claims,
                deployments: [{ subject: actor, id: deploymentId }],
            }),
            issuedAt * 1000,
        ),
    ).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "unexpected workload token identity",
    });
});
