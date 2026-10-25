import { TEST_START, testSimulation } from "@destack/test/world";
import type { Crypto } from "@destack/environment";
import { Failure } from "@destack/error";
import { expect, refusal, test } from "@destack/test";
import { Caller, KEY_ASSURANCE } from "../context/index.ts";
import { principal } from "../declare/index.ts";
import { PackageId } from "@destack/package";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Jwt, PublicKey } from "@destack/identity";
import { Assertion, AssertionVerifier, type AssertionKey } from "./assertion.ts";
import { MACHINE_KEY } from "./token.ts";

/** The package serving the token endpoint. */
const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000031");
/** The token endpoint every assertion has as its audience. */
const endpoint = "https://account.example/oauth2/token";

/** Generate a P-256 key pair. */
async function generate(crypto: Crypto): Promise<CryptoKeyPair> {
    return crypto.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]);
}

test("authenticate a machine by its assertion once, refusing replayed, expired, forged and foreign ones", async () => {
    // publish a machine's key, and verify assertions against it
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const pair = await generate(world.crypto);
    const forger = await generate(world.crypto);
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.machine.reference(Scope.universe.id, "machine-1"),
        publicKey: await PublicKey.of(pair.publicKey, world.crypto),
    };
    const verifier = new AssertionVerifier({
        crypto: world.crypto,
        endpoint,
        audience,
        keys: async (id) => (id === key.id ? key : undefined),
        spend: spending(),
    });
    const now = TEST_START;
    const signed = await Assertion.sign(pair.privateKey, key, endpoint, now, world);

    // sign a late, forged, unknown, foreign and untyped assertion
    const issuedAt = Math.floor(now / 1000);
    const foreign = "https://other.example/oauth2/token";
    const untyped = {
        jti: "untyped",
        iss: key.subject.id,
        sub: key.subject.id,
        aud: endpoint,
        iat: issuedAt,
        exp: issuedAt + 60,
    };
    const refused: [string, number][] = [
        [signed, now],
        [await Assertion.sign(pair.privateKey, key, endpoint, now, world), now + 120_000],
        [await Assertion.sign(forger.privateKey, key, endpoint, now, world), now],
        [await Assertion.sign(pair.privateKey, { ...key, id: "key-2" }, endpoint, now, world), now],
        [await Assertion.sign(pair.privateKey, key, foreign, now, world), now],
        [
            await Jwt.sign(untyped, { alg: "ES256", kid: key.id }, pair.privateKey, world.crypto),
            now,
        ],
    ];

    // verify the first once, then replay it and verify the others
    const authentication = await verifier.verify(signed, now);
    const refusals = [];
    for (const [token, at] of refused) {
        refusals.push(await refusal(verifier.verify(token, at)));
    }

    // answer the machine as the caller, and refuse every other assertion
    expect({ authentication, refusals }).toEqual({
        authentication: {
            caller: Caller.of(key.subject),
            credential: {
                kind: MACHINE_KEY,
                id: key.id,
                audience,
                verifiedAt: now,
                expiresAt: (Math.floor(now / 1000) + 60) * 1000,
                assurance: { level: KEY_ASSURANCE, authenticatedAt: now },
            },
        },
        refusals: [
            ["UNAUTHORIZED", "ASSERTION_REUSED", "assertion was already used"],
            ["UNAUTHORIZED", "ASSERTION_EXPIRED", "assertion expired"],
            ["UNAUTHORIZED", "INVALID_ASSERTION", "invalid assertion"],
            ["UNAUTHORIZED", "ASSERTION_KEY_UNKNOWN", "assertion has no standing key"],
            ["UNAUTHORIZED", "INVALID_ASSERTION", "invalid assertion"],
            ["UNAUTHORIZED", "INVALID_ASSERTION", "invalid assertion"],
        ],
    });
});

test("authenticate an installation within its space", async () => {
    // publish an installation's key and verify its assertion
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const pair = await generate(world.crypto);
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.installation.reference("space-1", "installation-1"),
        publicKey: await PublicKey.of(pair.publicKey, world.crypto),
    };
    const verifier = new AssertionVerifier({
        crypto: world.crypto,
        endpoint,
        audience,
        keys: async () => key,
        spend: spending(),
    });
    const now = TEST_START;
    const installation = await verifier.verify(
        await Assertion.sign(pair.privateKey, key, endpoint, now, world),
        now,
    );

    // bind the installation's credential to its space
    expect([
        installation.caller.subject,
        installation.credential.kind,
        installation.credential.scope,
    ]).toEqual([key.subject, "installation-key", "space-1"]);
});

test("authenticate a space and its installation by the space's key, refusing a subject the key cannot assert", async () => {
    // publish a space's key, found under the issuer its assertions claim
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const pair = await generate(world.crypto);
    const spaceId = Identifier.create("space", world);
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.space.reference(Scope.universe.id, spaceId),
        publicKey: await PublicKey.of(pair.publicKey, world.crypto),
    };
    const verifier = new AssertionVerifier({
        crypto: world.crypto,
        endpoint,
        audience,
        keys: async (id, issuer) => (id === key.id && issuer === spaceId ? key : undefined),
        spend: spending(),
    });

    // assert an installation of the space, the space itself, and a machine
    const now = TEST_START;
    const installationId = Identifier.create("installation", world);
    const sign = (subject: string) =>
        Assertion.sign(pair.privateKey, key, endpoint, now, world, subject);
    const installation = await verifier.verify(await sign(installationId), now);
    const itself = await verifier.verify(await sign(spaceId), now);
    const refused = await refusal(
        verifier.verify(await sign(Identifier.create("machine", world)), now),
    );

    // bind the installation to the space and let the space act as itself, refusing what its key cannot assert
    expect({
        installation: [
            installation.caller.subject,
            installation.credential.kind,
            installation.credential.scope,
        ],
        space: [itself.caller.subject, itself.credential.kind, itself.credential.scope],
        refused,
    }).toEqual({
        installation: [
            principal.installation.reference(spaceId, installationId),
            "installation-key",
            spaceId,
        ],
        space: [principal.space.reference(Scope.universe.id, spaceId), "space-key", undefined],
        refused: [
            "UNAUTHORIZED",
            "ASSERTION_OUTSIDE_AUTHORITY",
            "assertion subject is outside its key's authority",
        ],
    });
});

/** Spend digests once in memory, refusing a digest spent before as the proof store does. */
function spending(): (digest: string) => Promise<void> {
    const spent = new Set<string>();

    return async (digest) => {
        // refuse a digest spent before, and record this one
        if (spent.has(digest)) {
            throw Failure.error({
                code: "CONFLICT",
                reason: "DUPLICATE",
                domain: "@destack/object",
                message: "proof was used before",
            });
        }
        spent.add(digest);
    };
}
