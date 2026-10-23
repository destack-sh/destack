import { expect, refusal, test } from "@destack/test";
import { Scope, type Subject } from "@destack/sync";
import { exportJWK, generateKeyPair, SignJWT, type JWTPayload } from "jose";
import { ACCESS_TOKEN_TYPE, INSTALLATION_KEY, SPACE_KEY, TokenVerifier } from "./token.ts";
import { TokenIssuer } from "./issuer.ts";
import { universe } from "../index.ts";
import { type Authentication, Caller, Credential } from "../context/index.ts";
import { principal } from "../declare/index.ts";
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
        aud: PackageId.resource(audience),
        sub: subject.id,
        iat: issuedAt,
        exp: issuedAt + 60,
        jti: "token-example",
        client_id: "client-019f7480-0000-7000-8000-000000000009",
        space: spaceId,
        caller: {
            credential: { kind: "session", id: "session-example" },
            subject,
            groups: [],
            contacts: [],
        },
    };
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    const signed = await new SignJWT(claims)
        .setProtectedHeader({ alg: "ES256", kid: "current", typ: ACCESS_TOKEN_TYPE })
        .sign(keys.privateKey);
    const request = new Request("https://notes.example", {
        headers: { authorization: `Bearer ${signed}` },
    });
    const authentication = await verifier.authenticate(request, issuedAt * 1000, spaceId);
    expect(current(authentication, audience, issuedAt * 1000, spaceId).subject).toEqual(subject);
    expect(() => current(authentication, audience, issuedAt * 1000, "another-space")).toThrow(
        "caller authentication has a different audience or scope",
    );

    // accept an administrator impersonating a user and reject other impersonations
    const administrator: Subject = principal.user.reference("universe", "user-administrator");
    const impersonate = async (impersonator: Subject) =>
        verifier.authenticate(
            new Request(request, {
                headers: {
                    authorization: `Bearer ${await new SignJWT({ ...claims, act: [impersonator] })
                        .setProtectedHeader({
                            alg: "ES256",
                            kid: "current",
                            typ: ACCESS_TOKEN_TYPE,
                        })
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
    expect([impersonated.subject, impersonated.actors]).toEqual([subject, [administrator]]);
    for (const impersonator of [
        subject,
        principal.installation.reference("universe", "installation-example"),
    ]) {
        expect(await refusal(impersonate(impersonator))).toEqual([
            "UNAUTHORIZED",
            "INVALID_DELEGATION",
            "invalid token delegation",
        ]);
    }

    // reject changed claims signed by a trusted key
    const changedRefusals = [];
    for (const changed of [
        { iss: "https://other.example" },
        { aud: audience },
        {
            aud: PackageId.resource(
                PackageId.parse("package-019f7480-0000-7000-8000-000000000008"),
            ),
        },
        { sub: "another-user" },
        { exp: issuedAt },
        { iat: issuedAt + 10, exp: issuedAt + 60 },
        { exp: issuedAt + 61 },
        { client_id: "user-example-client" },
        { client_id: "machine-019f7480-0000-7000-8000-000000000009" },
        { jti: "" },
        { space: "space-019f7480-0000-7000-8000-000000000003" },
    ]) {
        const token = await new SignJWT({ ...claims, ...changed })
            .setProtectedHeader({ alg: "ES256", kid: "current", typ: ACCESS_TOKEN_TYPE })
            .sign(keys.privateKey);
        changedRefusals.push(
            await refusal(
                verifier.authenticate(
                    new Request(request, { headers: { authorization: `Bearer ${token}` } }),
                    issuedAt * 1000,
                    spaceId,
                ),
            ),
        );
    }
    // refuse the issuer, the audiences and the issue time by verifying the token, the rest by its claims
    const unverified = ["UNAUTHORIZED", "INVALID_TOKEN", "invalid access token"];
    const unclaimed = ["UNAUTHORIZED", "INVALID_TOKEN", "invalid access token claims"];
    expect(changedRefusals).toEqual([
        unverified,
        unverified,
        unverified,
        unclaimed,
        unclaimed,
        unverified,
        unclaimed,
        unclaimed,
        unclaimed,
        unclaimed,
        unclaimed,
    ]);

    // accept the audience among several, bound to the user's client, and refuse an untyped token
    const typed = async (payload: JWTPayload, typ: string) =>
        verifier.authenticate(
            new Request(request, {
                headers: {
                    authorization: `Bearer ${await new SignJWT(payload)
                        .setProtectedHeader({ alg: "ES256", kid: "current", typ })
                        .sign(keys.privateKey)}`,
                },
            }),
            issuedAt * 1000,
            spaceId,
        );
    const several = await typed(
        { ...claims, aud: [PackageId.resource(audience), "urn:destack:package:other"] },
        ACCESS_TOKEN_TYPE,
    );
    expect(several.credential.client).toBe(claims.client_id);
    expect(await refusal(typed(claims, "JWT"))).toEqual([
        "UNAUTHORIZED",
        "INVALID_TOKEN",
        "invalid access token",
    ]);

    // reject a token signed by an untrusted key
    const attacker = await generateKeyPair("ES256");
    const forged = await new SignJWT(claims)
        .setProtectedHeader({ alg: "ES256", kid: "current", typ: ACCESS_TOKEN_TYPE })
        .sign(attacker.privateKey);
    expect(
        await refusal(
            verifier.authenticate(
                new Request(request, { headers: { authorization: `Bearer ${forged}` } }),
                Date.now(),
                spaceId,
            ),
        ),
    ).toEqual(["UNAUTHORIZED", "INVALID_TOKEN", "invalid access token"]);
    expect(await refusal(verifier.authenticate(request, (issuedAt + 60) * 1000, spaceId))).toEqual([
        "UNAUTHORIZED",
        "AUTHENTICATION_EXPIRED",
        "caller authentication is not current",
    ]);

    // reuse cached public keys during an outage
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
    expect(await refusal(remote.authenticate(request, (issuedAt + 60) * 1000, spaceId))).toEqual([
        "UNAUTHORIZED",
        "AUTHENTICATION_EXPIRED",
        "caller authentication is not current",
    ]);

    // require discovery without cached public keys
    const disconnected = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: new URL(`${issuer}/jwks`),
        fetch: async () => new Response(null, { status: 503 }),
    });
    expect(await refusal(disconnected.authenticate(request, issuedAt * 1000, spaceId))).toEqual([
        "SERVICE_UNAVAILABLE",
        "KEYS_UNAVAILABLE",
        "authentication keys are unavailable",
    ]);

    // sign and verify workload identity, delegates and their grants
    const actor: Subject = principal.installation.reference(spaceId, "software-example");
    const deploymentId = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000004");
    const grant = {
        id: schema.identifier("grant").parse("grant-019f7480-0000-7000-8000-000000000005"),
        scope: subject.id,
    };
    const delegated = {
        caller: {
            ...authentication.caller,
            actors: [actor],
            grants: [grant],
            contacts: [{ medium: "email" as const, address: "alice@example.com" }],
        },
        credential: {
            ...authentication.credential,
            deployments: [{ subject: actor, id: deploymentId }],
            assurance: { level: 2, authenticatedAt: issuedAt * 1000 },
        },
    };
    const signing = {
        authority: { kind: "universe" as const },
        issuer,
        sign: async (payload: JWTPayload, header: { readonly typ: string }) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current", ...header })
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
        deployments: represented.credential.deployments,
        actors: access.actors,
        grants: access.grants,
        principal: Caller.principal(access),
        assurance: represented.credential.assurance,
        contacts: Caller.contacts(access),
    }).toEqual({
        deployments: [{ subject: actor, id: deploymentId }],
        actors: [actor],
        grants: [grant],
        principal: actor,
        assurance: { level: 2, authenticatedAt: issuedAt * 1000 },
        contacts: [],
    });

    // record a deployment for each workload in a delegation chain
    const secondActor = { ...actor, id: "second-software" };
    const secondDeployment = schema
        .identifier("deployment")
        .parse("deployment-019f7480-0000-7000-8000-000000000005");
    const chain = {
        caller: { ...delegated.caller, actors: [actor, secondActor] },
        credential: {
            ...delegated.credential,
            deployments: [
                { subject: actor, id: deploymentId },
                { subject: secondActor, id: secondDeployment },
            ],
        },
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
        chainedCaller.credential.deployments,
        chainedAccess.actors,
        Caller.sender(chainedAccess),
    ]).toEqual([chain.credential.deployments, [actor, secondActor], secondActor]);
    expect(
        await refusal(
            authority.issue(
                {
                    ...chain,
                    credential: {
                        ...chain.credential,
                        deployments: [
                            { subject: secondActor, id: secondDeployment },
                            { subject: secondActor, id: deploymentId },
                        ],
                    },
                },
                issuedAt * 1000,
            ),
        ),
    ).toEqual(["UNAUTHORIZED", "INVALID_WORKLOAD_IDENTITY", "invalid workload token identity"]);

    // reject malformed chains signed outside TokenIssuer, and a delegate acting without a grant unless a person impersonates
    const chainRefusals = [];
    for (const act of [
        [],
        [actor, actor],
        [subject, actor],
        [{ ...actor, type: "space" }],
        [secondActor],
    ]) {
        const token = await signing.sign(
            {
                ...claims,
                exp: issuedAt + 30,
                act,
                caller: {
                    ...claims.caller,
                    credential: {
                        ...claims.caller.credential,
                        deployments: [{ subject: actor, id: deploymentId }],
                    },
                },
            },
            { typ: ACCESS_TOKEN_TYPE },
        );
        chainRefusals.push(
            await refusal(
                verifier.authenticate(
                    new Request(request, {
                        headers: { authorization: `Bearer ${token}` },
                    }),
                    issuedAt * 1000,
                    spaceId,
                ),
            ),
        );
    }
    // refuse an empty chain by its claims, a repeated or self chain as a delegation, and an unexpected actor as identity
    const delegation = ["UNAUTHORIZED", "INVALID_DELEGATION", "invalid token delegation"];
    const identity = [
        "UNAUTHORIZED",
        "INVALID_WORKLOAD_IDENTITY",
        "unexpected workload token identity",
    ];
    expect(chainRefusals).toEqual([
        ["UNAUTHORIZED", "INVALID_TOKEN", "invalid access token claims"],
        delegation,
        delegation,
        identity,
        identity,
    ]);

    // reject space authorities delegating for global users
    const local = new TokenIssuer({ ...signing, authority: { kind: "space", space: spaceId } });
    expect(await refusal(local.issue(delegated, issuedAt * 1000))).toEqual([
        "UNAUTHORIZED",
        "TOKEN_OUTSIDE_AUTHORITY",
        "token exceeds issuer authority",
    ]);
    const localVerifier = new TokenVerifier({
        authority: { kind: "space", space: spaceId },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    expect(await refusal(localVerifier.authenticate(request, issuedAt * 1000, spaceId))).toEqual([
        "UNAUTHORIZED",
        "TOKEN_OUTSIDE_AUTHORITY",
        "token exceeds issuer authority",
    ]);

    // accept only the space's deployed software
    const workload = {
        caller: Caller.of(actor),
        credential: {
            ...authentication.credential,
            kind: "workload",
            id: "credential-example",
            deployments: [{ subject: actor, id: deploymentId }],
        },
    };
    const workloadToken = await local.issue(workload, issuedAt * 1000);
    const received = await localVerifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${workloadToken.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    expect(received.caller.subject).toEqual(actor);
    expect(received.credential.deployments).toEqual([{ subject: actor, id: deploymentId }]);
    const { deployments: _deployments, ...undeployed } = workload.credential;
    const withoutDeployments = { ...workload, credential: undeployed };
    expect(await refusal(local.issue(withoutDeployments, issuedAt * 1000))).toEqual([
        "UNAUTHORIZED",
        "INVALID_WORKLOAD_IDENTITY",
        "invalid workload token identity",
    ]);

    // accept the installation its key authenticated without a deployment
    const keyed = {
        ...withoutDeployments,
        credential: {
            ...undeployed,
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
    expect([keyedCaller.caller.subject, keyedCaller.credential.deployments]).toEqual([
        actor,
        undefined,
    ]);
    expect(
        await refusal(
            local.issue(
                {
                    ...keyed,
                    credential: {
                        ...keyed.credential,
                        deployments: [{ subject: actor, id: deploymentId }],
                    },
                },
                issuedAt * 1000,
            ),
        ),
    ).toEqual(["UNAUTHORIZED", "INVALID_WORKLOAD_IDENTITY", "unexpected workload token identity"]);

    // accept the installation its space's key authenticated without a deployment, as its host forwards its writes
    const spaceKeyed = {
        ...withoutDeployments,
        credential: { ...undeployed, kind: SPACE_KEY, id: spaceId },
    };
    const spaceKeyedToken = await local.issue(spaceKeyed, issuedAt * 1000);
    const spaceKeyedCaller = await localVerifier.authenticate(
        new Request(request, {
            headers: { authorization: `Bearer ${spaceKeyedToken.accessToken}` },
        }),
        issuedAt * 1000,
        spaceId,
    );
    expect([spaceKeyedCaller.caller.subject, spaceKeyedCaller.credential.deployments]).toEqual([
        actor,
        undefined,
    ]);
});

test("issue the universe a token of its key, verified without a separate client", async () => {
    // sign as the universe's authority
    const keys = await generateKeyPair("ES256");
    const publicKey = { ...(await exportJWK(keys.publicKey)), kid: "current", alg: "ES256" };
    const issuer = "https://account.example";
    const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const authority = new TokenIssuer({
        authority: { kind: "universe" },
        issuer,
        sign: async (payload: JWTPayload, header: { readonly typ: string }) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current", ...header })
                .sign(keys.privateKey),
    });

    // issue a token to the universe calling a package as itself
    const now = Date.now();
    const subject: Subject = universe.reference(Scope.universe.id, Scope.universe.id);
    const issued = await authority.issue(
        {
            caller: Caller.of(subject),
            credential: {
                kind: "universe-key",
                id: Scope.universe.id,
                audience,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        },
        now,
    );

    // verify it as the universe with no separate client
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience,
        keys: { keys: [publicKey] },
    });
    const verified = await verifier.verify(issued.accessToken, now);
    expect([verified.caller.subject, verified.credential.client]).toEqual([subject, undefined]);
});

/** Require an authentication current at a time in a scope, and answer its caller. */
function current(
    authentication: Authentication,
    audience: PackageId,
    now: number,
    scope: string,
): Caller {
    Credential.requireCurrent(authentication.credential, audience, now, scope);

    return authentication.caller;
}
