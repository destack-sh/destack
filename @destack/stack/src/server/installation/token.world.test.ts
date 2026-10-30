import type { Crypto } from "@destack/environment";
import { principal } from "@destack/access";
import { nonce } from "@destack/access/token";
import { Jwk, Jwt, type JwtClaims, PublicKey } from "@destack/identity";
import { Digest, Identifier, present, Uuid } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Assertion, ASSERTION_TYPE, INSTALLATION_KEY, TokenVerifier } from "@destack/access/token";
import { createClient } from "@destack/object/client";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { Identity } from "@destack/account/object";
import objectPackage from "@destack/object/package";
import stackPackage from "../../package.ts";
import { ids, TestSpace } from "../../test/index.ts";

/** The audience of installations' assertions: the object package granting their tokens. */
const ENDPOINT = PackageId.resource(objectPackage.id);

/** The Ed25519 key parameters of an installation's signing key (RFC 8037). */
const EDWARDS = { name: "Ed25519" } as const;

/** The P-256 key parameters of an installation's signing key (RFC 7518 3.4). */
const CURVE = { name: "ECDSA", namedCurve: "P-256" };

/** Grant tokens to an installation asking by assertions its registered keys sign once each. */
test("grant an installation tokens of its space as it asks by assertions its keys sign once each, until a key is revoked", async () => {
    // add the space with its identity and keys and an enabled installation in it, at a paused time
    const fixture = await TestSpace.open();
    const { simulation } = fixture;
    const { database } = fixture;
    const { environment } = fixture.machine;
    const now = simulation.clock.now();

    // run the calls in the space's world
    await (async () => {
        const served = await fixture.serve();
        const owner = createClient(
            stackPackage,
            served.as(fixture.owner.subject).endpoint(stackPackage),
            environment,
        );
        const installer = createClient(
            objectPackage,
            served.as(fixture.owner.subject).endpoint(objectPackage),
            environment,
        );
        const { id: notes } = await installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            packageId: ids.package,
            selection: { kind: "release", version: "2026.9.0" },
            name: "notes",
        });

        // register an Ed25519 and a P-256 key of the installation and refuse a point off its curve
        const { crypto } = environment;
        const edwards = await crypto.generateKey(EDWARDS, true, ["sign", "verify"]);
        const curve = await crypto.generateKey(CURVE, true, ["sign", "verify"]);
        const register = async (name: string, publicKey: unknown) =>
            owner.installationKey.create({
                spaceId: ids.space,
                parentId: notes,
                requestId: RequestId.create(fixture.machine.environment),
                name,
                publicKey: PublicKey.parse(publicKey),
            });
        const { kty, crv, x } = await Jwk.export(edwards.publicKey, crypto);
        const ci = await register("ci", { kty, crv, x });
        const exported = await Jwk.export(curve.publicKey, crypto);
        const point = {
            kty: exported.kty,
            crv: exported.crv,
            x: exported["x"],
            y: exported["y"],
        };
        const serverKeyRow = await register("server", point);
        expect(
            await refusal(register("broken", { ...point, x: "A".repeat(43), y: "A".repeat(43) })),
        ).toEqual(["BAD_REQUEST", "KEY_INVALID", "invalid public key"]);

        // grant a token of only the installation for each key's assertion and record the key's use
        const installation = principal.installation.reference(ids.space, notes);
        const grant = (assertion: string) =>
            createClient(
                objectPackage,
                {
                    url: "http://space.test",
                    headers: { authorization: `Bearer ${assertion}` },
                    fetch: (request) => served.server(objectPackage).fetch(request),
                },
                environment,
            ).installation.token({
                scope: ids.space,
                id: notes,
                requestId: RequestId.create(fixture.machine.environment),
                audience: ids.package,
            });
        const keyOf = (id: string, publicKey: unknown) => ({
            id,
            subject: installation,
            publicKey: PublicKey.parse(publicKey),
        });
        const ciKey = keyOf(ci.id, { kty, crv, x });
        const ciAssertion = await Assertion.sign(
            edwards.privateKey,
            ciKey,
            ENDPOINT,
            now,
            environment,
        );
        const ciToken = await grant(ciAssertion);
        await fixture.simulation.idle();
        const keys = await Identity.keys(fixture.machine.directory.snapshot, ids.space, crypto);
        const verifier = new TokenVerifier({
            authority: { kind: "space", space: ids.space },
            issuer: ids.space,
            audience: ids.package,
            keys,
            environment,
        });
        const verified = await verifier.verify(ciToken.accessToken, now, ids.space);
        const { credential } = verified;
        expect([
            verified.caller.subject,
            { kind: credential.kind, id: credential.id },
            credential.deployments,
        ]).toEqual([installation, { kind: INSTALLATION_KEY, id: ci.id }, undefined]);
        const serverKey = keyOf(serverKeyRow.id, serverKeyRow.publicKey);
        const serverAssertion = await Assertion.sign(
            curve.privateKey,
            serverKey,
            ENDPOINT,
            now,
            environment,
        );
        await grant(serverAssertion);
        const used = await owner.installationKey.get({ spaceId: ids.space, id: ci.id });
        expect(used.lastUsedAt).toBe(now);

        // refuse a replayed assertion, and assertions without an identifier, for another package, of another key or living past five minutes
        const foreign = Identifier.create("package", fixture.environment);
        const issuedAt = Math.floor(now / 1000);
        const unsigned = (claims: JwtClaims, keyId: string = ci.id) =>
            Jwt.sign(
                {
                    iss: notes,
                    sub: notes,
                    aud: ENDPOINT,
                    iat: issuedAt,
                    exp: issuedAt + 60,
                    ...claims,
                },
                { alg: "Ed25519", kid: keyId, typ: ASSERTION_TYPE },
                edwards.privateKey,
                crypto,
            );
        expect([
            await refusal(grant(ciAssertion)),
            await refusal(grant(await unsigned({}))),
            await refusal(
                grant(
                    await Assertion.sign(
                        edwards.privateKey,
                        ciKey,
                        PackageId.resource(PackageId.parse(foreign)),
                        now,
                        environment,
                    ),
                ),
            ),
            await refusal(
                grant(await unsigned({ jti: Uuid.v4(environment.random) }, serverKeyRow.id)),
            ),
            await refusal(
                grant(
                    await unsigned({
                        jti: Uuid.v4(environment.random),
                        exp: issuedAt + 600,
                    }),
                ),
            ),
        ]).toEqual([
            ["UNAUTHORIZED", "ASSERTION_REUSED", "assertion was already used"],
            ["UNAUTHORIZED", "ASSERTION_INVALID", "invalid assertion"],
            ["UNAUTHORIZED", "ASSERTION_INVALID", "invalid assertion"],
            ["UNAUTHORIZED", "ASSERTION_INVALID", "invalid assertion"],
            ["UNAUTHORIZED", "ASSERTION_TOO_LONG", "assertion lifetime exceeds five minutes"],
        ]);

        // record each spent assertion once as the space's nonce, until the assertion expires
        const spent = [
            await spentProof(ci.id, ciAssertion, crypto),
            await spentProof(serverKey.id, serverAssertion, crypto),
        ];
        const kept = await database
            .select({ id: nonce.id, expiresAt: nonce.expiresAt })
            .from(nonce)
            .orderBy(nonce.id);
        expect(kept).toEqual(spent.toSorted((left, right) => left.id.localeCompare(right.id)));

        // refuse every assertion of a revoked key
        await owner.installationKey.revoke({
            spaceId: ids.space,
            id: ci.id,
            requestId: RequestId.create(fixture.machine.environment),
        });
        expect(
            await refusal(
                grant(await Assertion.sign(edwards.privateKey, ciKey, ENDPOINT, now, environment)),
            ),
        ).toEqual(["UNAUTHORIZED", "ASSERTION_KEY_UNKNOWN", "assertion has no standing key"]);
    })();
});

/** Read the nonce an assertion a key signed spends: its digest and its expiry. */
async function spentProof(keyId: string, assertion: string, crypto: Crypto) {
    const { jti, exp } = Jwt.claims(assertion);

    return {
        id: await Digest.of(JSON.stringify([keyId, jti]), crypto),
        expiresAt: present(exp, "the assertion's expiry") * 1000,
    };
}
