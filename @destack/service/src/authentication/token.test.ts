import { expect, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { exportJWK, generateKeyPair, SignJWT, type JWTPayload } from "jose";
import { INSTALLATION_KEY, SPACE_KEY, TokenVerifier } from "./token.ts";
import { TokenIssuer } from "./issuer.ts";
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
    const request = new Request("https://service.example", {
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
            issuedAt * 1000,
            spaceId,
        );
    const impersonated = current(
        await impersonate(administrator),
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

    // keep workload identity and delegates through signing and verification
    const actor: Subject = principal.installation.reference(spaceId, "software-example");
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000004");
    const read = { packageId: PackageId.parse(audience), type: "note", name: "read" };
    const selection = { ...read, scope: spaceId, objectId: "note-one" };
    const delegated: Caller = {
        ...caller,
        deployments: [{ subject: actor, id: deploymentId }],
        delegates: [{ subject: actor, authority: "lent" }],
        permissions: [selection],
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
    expect(represented.deployments).toEqual([{ subject: actor, id: deploymentId }]);
    expect(represented.delegates).toEqual([{ subject: actor, authority: "lent" }]);
    const access = current(represented, audience, issuedAt * 1000, spaceId);
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
    const chain: Caller = {
        ...delegated,
        deployments: [
            { subject: actor, id: deploymentId },
            { subject: secondActor, id: secondDeployment },
        ],
        delegates: [
            { subject: actor, authority: "lent" },
            { subject: secondActor, authority: "lent" },
        ],
    };
    const chained = await authority.issue(chain, issuedAt * 1000);
    const chainedCaller = await verifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${chained.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    expect(chainedCaller.deployments).toEqual(chain.deployments);
    expect(Caller.delegation(current(chainedCaller, audience, issuedAt * 1000, spaceId))).toEqual([
        { delegate: actor, delegator: subject, authority: "lent" },
        { delegate: secondActor, delegator: actor, authority: "lent" },
    ]);
    await expect(
        authority.issue(
            { ...chain, deployments: [{ subject: secondActor, id: secondDeployment }] },
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

/** Issue tokens as a space with a fresh key. */
async function spaceIssuer(spaceId: string) {
    const keys = await generateKeyPair("ES256");
    const issuer = new TokenIssuer({
        issuer: spaceId,
        authority: { kind: "space", space: spaceId },
        sign: (payload: JWTPayload) =>
            new SignJWT(payload).setProtectedHeader({ alg: "ES256" }).sign(keys.privateKey),
    });

    return { issuer, keys };
}

test("exchange a person's call for an hour's token lending their authority to an installation, valid in its issuing space alone", async () => {
    // sign as the person's home
    const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const [home, other] = [
        "space-019f7480-0000-7000-8000-00000000000a",
        "space-019f7480-0000-7000-8000-00000000000b",
    ];
    const person: Subject = principal.user.reference("universe", "user-example");
    const now = Date.now();
    const issuing = await spaceIssuer(home);
    const actor = principal.installation.reference(home, "installation-example");
    const token = await issuing.issuer.exchange(
        {
            subject: person,
            subjects: [person],
            credential: {
                kind: "session",
                id: "session-example",
                audience,
                scope: home,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        },
        { actor, audience },
        now,
    );
    const verifier = new TokenVerifier({
        authority: { kind: "space", space: home },
        issuer: home,
        audience,
        keys: { keys: [{ ...(await exportJWK(issuing.keys.publicKey)), alg: "ES256" }] },
    });

    // accept it in the home for its hour, and refuse it past the hour and in another space
    const acting = current(
        await verifier.verify(token, now + 59 * 60_000, home),
        audience,
        now + 59 * 60_000,
        home,
    );
    expect([acting.subject, acting.delegates]).toEqual([
        person,
        [{ subject: actor, authority: "lent" }],
    ]);
    await expect(verifier.verify(token, now + 61 * 60_000, home)).rejects.toThrow();
    await expect(verifier.verify(token, now, other)).rejects.toThrow();
});

/** Require a verified caller current for an audience at a time in a scope, and answer it. */
function current(caller: Caller, audience: PackageId, now: number, scope: string): Caller {
    Caller.requireCurrent(caller, audience, now, scope);

    return caller;
}
