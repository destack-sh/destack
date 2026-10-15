import { principal } from "@destack/access";
import { PublicKey } from "@destack/identity";
import { LocalKeyring, MemoryKeychain } from "@destack/identity";
import { Clock, Digest, Identifier, present } from "@destack/schema";
import { PackageId } from "@destack/package";
import {
    Assertion,
    ASSERTION_TYPE,
    INSTALLATION_KEY,
    TokenVerifier,
} from "@destack/runtime/authentication";
import { createClient } from "@destack/runtime/client";
import { RequestId } from "@destack/runtime/request";
import { expect, refusal, test, TestClock } from "@destack/test";
import {
    createLocalJWKSet,
    decodeJwt,
    exportJWK,
    generateKeyPair,
    SignJWT,
    type JWTPayload,
} from "jose";
import * as object from "../../object/index.ts";
import { IdentityKeystore } from "@destack/account/identity";
import { Identity } from "@destack/account/object";
import { Snapshot } from "@destack/db";
import stackPackage from "../../package.ts";
import { ids, openUniverse, StackFixture } from "../../test/index.ts";
import { TestDirectory } from "@destack/account/test";

/** The audience of installations' assertions: the stack package granting their tokens. */
const ENDPOINT = PackageId.resource(stackPackage.id);

/** Grant tokens to an installation asking by assertions its registered keys sign once each. */
test("grant an installation tokens of its space as it asks by assertions its keys sign once each, until a key is revoked", async () => {
    // keep the space with its identity and keys and an enabled installation in it
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const opened = await openUniverse();
    const placed = { id: ids.space, scope: ids.account, machine: ids.cloud, epoch: 1 };
    await TestDirectory.place(opened.universe, placed);
    const keystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), ids.cloud),
        opened.identities,
        Clock.system,
    );
    await keystore.generate(database, placed.id);
    const now = Date.now();
    await database.insert(object.installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
    const { server } = await fixture.server({ ...opened, keystore, clock: new TestClock(now) });
    const owner = createClient(stackPackage, StackFixture.endpoint(server, ids.owner));

    // register an Ed25519 and a P-256 key of the installation and refuse a point off its curve
    const edwards = await generateKeyPair("Ed25519");
    const curve = await generateKeyPair("ES256");
    const register = async (name: string, publicKey: unknown) =>
        owner.installationKey.create({
            spaceId: ids.space,
            parentId: ids.notes,
            requestId: RequestId.create(Date.now()),
            name,
            publicKey: PublicKey.parse(publicKey),
        });
    const { kty, crv, x } = await exportJWK(edwards.publicKey);
    const ci = await register("ci", { kty, crv, x });
    const exported = await exportJWK(curve.publicKey);
    const serverKeyRow = await register("server", {
        kty: exported.kty,
        crv: exported.crv,
        x: exported.x,
        y: exported.y,
    });
    expect(
        await refusal(register("broken", { ...exported, x: "A".repeat(43), y: "A".repeat(43) })),
    ).toEqual(["BAD_REQUEST", "invalid public key"]);

    // grant a token of only the installation for each key's assertion and record the key's use
    const installation = principal.installation.reference(ids.space, ids.notes);
    const grant = (assertion: string) =>
        createClient(stackPackage, {
            url: "http://space.test",
            headers: { authorization: `Bearer ${assertion}` },
            fetch: (request) => server.fetch(request),
        }).installation.token({
            spaceId: ids.space,
            id: ids.notes,
            requestId: RequestId.create(Date.now()),
            audience: ids.package,
        });
    const keyOf = (id: string, publicKey: unknown) => ({
        id,
        subject: installation,
        publicKey: PublicKey.parse(publicKey),
    });
    const ciKey = keyOf(ci.id, { kty, crv, x });
    const ciAssertion = await Assertion.sign(edwards.privateKey, ciKey, ENDPOINT, now);
    const ciToken = await grant(ciAssertion);
    const keys = await Identity.keys(Snapshot.live(opened.universe), ids.space);
    const verifier = new TokenVerifier({
        authority: { kind: "space", space: ids.space },
        issuer: ids.space,
        audience: ids.package,
        keys: createLocalJWKSet(keys),
    });
    const verified = await verifier.verify(ciToken.accessToken, now, ids.space);
    const { credential } = verified;
    expect([
        verified.caller.subject,
        { kind: credential.kind, id: credential.id },
        credential.deployments,
    ]).toEqual([installation, { kind: INSTALLATION_KEY, id: ci.id }, undefined]);
    const serverKey = keyOf(serverKeyRow.id, serverKeyRow.publicKey);
    const serverAssertion = await Assertion.sign(curve.privateKey, serverKey, ENDPOINT, now);
    await grant(serverAssertion);
    const used = await owner.installationKey.get({ spaceId: ids.space, id: ci.id });
    expect(used.lastUsedAt).toBe(now);

    // refuse a replayed assertion, and assertions without an identifier, for another package, of another key or living past five minutes
    const foreign = Identifier.create("package");
    const issuedAt = Math.floor(now / 1000);
    const unsigned = (claims: JWTPayload, keyId: string = ci.id) =>
        new SignJWT({
            iss: ids.notes,
            sub: ids.notes,
            aud: ENDPOINT,
            iat: issuedAt,
            exp: issuedAt + 60,
            ...claims,
        })
            .setProtectedHeader({ alg: "Ed25519", kid: keyId, typ: ASSERTION_TYPE })
            .sign(edwards.privateKey);
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
                ),
            ),
        ),
        await refusal(grant(await unsigned({ jti: crypto.randomUUID() }, serverKeyRow.id))),
        await refusal(grant(await unsigned({ jti: crypto.randomUUID(), exp: issuedAt + 600 }))),
    ]).toEqual([
        ["UNAUTHORIZED", "assertion was used before"],
        ["UNAUTHORIZED", "invalid assertion"],
        ["UNAUTHORIZED", "invalid assertion"],
        ["UNAUTHORIZED", "invalid assertion"],
        ["UNAUTHORIZED", "assertion lifetime exceeds five minutes"],
    ]);

    // keep each spent assertion once as the space's proof, until the assertion expires
    const spent = [
        await spentProof(ci.id, ciAssertion),
        await spentProof(serverKey.id, serverAssertion),
    ];
    const kept = await database
        .select({ id: object.proof.table.id, expiresAt: object.proof.table.expiresAt })
        .from(object.proof.table)
        .orderBy(object.proof.table.id);
    expect(kept).toEqual(spent.toSorted((left, right) => left.id.localeCompare(right.id)));

    // refuse every assertion of a revoked key
    await owner.installationKey.revoke({
        spaceId: ids.space,
        id: ci.id,
        requestId: RequestId.create(Date.now()),
    });
    expect(
        await refusal(grant(await Assertion.sign(edwards.privateKey, ciKey, ENDPOINT, now))),
    ).toEqual(["UNAUTHORIZED", "assertion has no standing key"]);
});

/** Read the proof an assertion a key signed spends: its digest and its expiry. */
async function spentProof(keyId: string, assertion: string) {
    const { jti, exp } = decodeJwt(assertion);

    return {
        id: await Digest.of(JSON.stringify([keyId, jti])),
        expiresAt: present(exp, "the assertion's expiry") * 1000,
    };
}
