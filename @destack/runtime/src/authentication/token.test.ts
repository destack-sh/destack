import { expect, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { exportJWK, generateKeyPair, SignJWT, type JWTPayload } from "jose";
import { INSTALLATION_KEY, SPACE_KEY, TokenVerifier } from "./token.ts";
import { TokenIssuer } from "./issuer.ts";
import { principal, GrantPermission, Caller } from "@destack/access";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** Verify signed identity and reject cross-package, cross-space and stale authority. */
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
        space: spaceId,
        caller: {
            credential: { kind: "session", id: "session-example" },
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
    const request = new Request("https://notes.example", {
        headers: { authorization: `Bearer ${signed}` },
    });
    const caller = await verifier.authenticate(request, issuedAt * 1000, spaceId);
    expect(current(caller, audience, issuedAt * 1000, spaceId).subject).toEqual(subject);
    expect(() => current(caller, audience, issuedAt * 1000, "another-space")).toThrow(
        "caller authentication has a different audience or scope",
    );

    // accept an administrator impersonating a user and reject other impersonations
    const administrator: Subject = principal.user.reference("universe", "user-administrator");
    const impersonate = async (impersonator: Subject) =>
        verifier.authenticate(
            new Request(request, {
                headers: {
                    authorization: `Bearer ${await new SignJWT({ ...claims, act: [impersonator] })
                        .setProtectedHeader({ alg: "ES256", kid: "current" })
                        .sign(keys.privateKey)}`,
                },
            }),
            issuedAt * 1000,
            spaceId,
        );
    const impersonated = current(
        await impersonate(administrator),
        audience,
        issuedAt * 1000,
        spaceId,
    );
    expect([impersonated.subject, impersonated.delegates]).toEqual([subject, [administrator]]);
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
        { space: "space-019f7480-0000-7000-8000-000000000003" },
    ]) {
        const token = await new SignJWT({ ...claims, ...changed })
            .setProtectedHeader({ alg: "ES256", kid: "current" })
            .sign(keys.privateKey);
        await expect(
            verifier.authenticate(
                new Request(request, { headers: { authorization: `Bearer ${token}` } }),
                issuedAt * 1000,
                spaceId,
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
            Date.now(),
            spaceId,
        ),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    await expect(
        verifier.authenticate(request, (issuedAt + 60) * 1000, spaceId),
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
    await remote.authenticate(request, issuedAt * 1000, spaceId);
    isAvailable = false;
    await remote.authenticate(request, (issuedAt + 30) * 1000, spaceId);
    expect(reads).toBe(1);
    await expect(
        remote.authenticate(request, (issuedAt + 60) * 1000, spaceId),
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
        disconnected.authenticate(request, issuedAt * 1000, spaceId),
    ).rejects.toMatchObject({ code: "SERVICE_UNAVAILABLE" });

    // keep workload identity, delegates and their grant through signing and verification
    const actor: Subject = principal.installation.reference(spaceId, "software-example");
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000004");
    const read = { packageId: PackageId.parse(audience), type: "note", name: "read" };
    const selection = { ...read, scope: spaceId, objectId: "note-one" };
    const grant = {
        id: schema.identifier("grant").parse("grant-019f7480-0000-7000-8000-000000000005"),
        scope: subject.id,
        permissions: [selection],
    };
    const delegated: Caller = {
        ...caller,
        deployments: [{ subject: actor, id: deploymentId }],
        delegates: [actor],
        grant,
        assurance: { level: 2, authenticatedAt: issuedAt * 1000 },
        contacts: [{ medium: "email", address: "alice@example.com" }],
    };
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
        issuedAt * 1000,
        spaceId,
    );
    const access = current(represented, audience, issuedAt * 1000, spaceId);
    expect({
        deployments: access.deployments,
        delegates: access.delegates,
        grant: access.grant,
        principal: Caller.principal(access),
        assurance: access.assurance,
        contacts: Caller.contacts(access),
        write: GrantPermission.allows(
            { ...read, name: "write" },
            { scope: spaceId, id: "note-one" },
            access.grant?.permissions,
        ),
    }).toEqual({
        deployments: [{ subject: actor, id: deploymentId }],
        delegates: [actor],
        grant,
        principal: actor,
        assurance: { level: 2, authenticatedAt: issuedAt * 1000 },
        contacts: [],
        write: false,
    });

    // keep a deployment for each workload in a delegation chain
    const secondActor = { ...actor, id: "second-software" };
    const secondDeployment = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000005");
    const chain: Caller = {
        ...delegated,
        deployments: [
            { subject: actor, id: deploymentId },
            { subject: secondActor, id: secondDeployment },
        ],
        delegates: [actor, secondActor],
    };
    const chained = await authority.issue(chain, issuedAt * 1000);
    const chainedCaller = await verifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${chained.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    const chainedAccess = current(chainedCaller, audience, issuedAt * 1000, spaceId);
    expect([
        chainedAccess.deployments,
        chainedAccess.delegates,
        Caller.sender(chainedAccess),
    ]).toEqual([chain.deployments, [actor, secondActor], secondActor]);
    await expect(
        authority.issue(
            {
                ...chain,
                deployments: [
                    { subject: secondActor, id: secondDeployment },
                    { subject: secondActor, id: deploymentId },
                ],
            },
            issuedAt * 1000,
        ),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "invalid workload token identity" });

    // reject malformed chains signed outside TokenIssuer, and a delegate acting without a grant unless a person impersonates
    for (const act of [
        [],
        [actor, actor],
        [subject, actor],
        [{ ...actor, type: "space" }],
        [secondActor],
    ]) {
        const token = await signing.sign({
            ...claims,
            exp: issuedAt + 30,
            act,
            caller: { ...claims.caller, deployments: [{ subject: actor, id: deploymentId }] },
        });
        await expect(
            verifier.authenticate(
                new Request(request, {
                    headers: { authorization: `Bearer ${token}` },
                }),
                issuedAt * 1000,
                spaceId,
            ),
        ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    }

    // reject space authorities delegating for global users
    const local = new TokenIssuer({ ...signing, authority: { kind: "space", space: spaceId } });
    await expect(local.issue(delegated, issuedAt * 1000)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
    });
    const localVerifier = new TokenVerifier({
        authority: { kind: "space", space: spaceId },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    await expect(
        localVerifier.authenticate(request, issuedAt * 1000, spaceId),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // accept only the space's own deployed software
    const workload: Caller = {
        ...caller,
        credential: {
            ...Caller.requireCredential(caller),
            kind: "workload",
            id: "credential-example",
        },
        subject: actor,
        subjects: [actor],
        deployments: [{ subject: actor, id: deploymentId }],
    };
    const workloadToken = await local.issue(workload, issuedAt * 1000);
    const received = await localVerifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${workloadToken.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    expect(received.subject).toEqual(actor);
    expect(received.deployments).toEqual([{ subject: actor, id: deploymentId }]);
    const { deployments: _deployments, ...withoutDeployments } = workload;
    await expect(local.issue(withoutDeployments, issuedAt * 1000)).rejects.toMatchObject({
        code: "UNAUTHORIZED",
    });

    // accept the installation its own key authenticated without a deployment
    const keyed: Caller = {
        ...withoutDeployments,
        credential: {
            ...Caller.requireCredential(caller),
            kind: INSTALLATION_KEY,
            id: "installation-key-example",
        },
    };
    const keyedToken = await local.issue(keyed, issuedAt * 1000);
    const keyedCaller = await localVerifier.authenticate(
        new Request(request, { headers: { authorization: `Bearer ${keyedToken.accessToken}` } }),
        issuedAt * 1000,
        spaceId,
    );
    expect([keyedCaller.subject, keyedCaller.deployments]).toEqual([actor, undefined]);
    await expect(
        local.issue(
            { ...keyed, deployments: [{ subject: actor, id: deploymentId }] },
            issuedAt * 1000,
        ),
    ).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        message: "unexpected workload token identity",
    });

    // accept the installation its space's key authenticated without a deployment, as its host forwards its writes
    const spaceKeyed: Caller = {
        ...withoutDeployments,
        credential: { ...Caller.requireCredential(caller), kind: SPACE_KEY, id: spaceId },
    };
    const spaceKeyedToken = await local.issue(spaceKeyed, issuedAt * 1000);
    const spaceKeyedCaller = await localVerifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${spaceKeyedToken.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    expect([spaceKeyedCaller.subject, spaceKeyedCaller.deployments]).toEqual([actor, undefined]);
});

/** Require a verified caller current for an audience at a time in a scope, and answer it. */
function current(caller: Caller, audience: PackageId, now: number, scope: string): Caller {
    Caller.requireCurrent(caller, audience, now, scope);

    return caller;
}
