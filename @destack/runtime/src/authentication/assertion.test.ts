import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { KEY_ASSURANCE, principal } from "@destack/access";
import { PublicKey } from "@destack/identity";
import { PackageId } from "@destack/package";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { SignJWT } from "jose";
import { Assertion, AssertionVerifier, proofReplay, type AssertionKey } from "./assertion.ts";
import { MACHINE_KEY } from "./token.ts";

/** The package serving the token endpoint. */
const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000031");
/** The token endpoint every assertion has as its audience. */
const endpoint = "https://account.example/oauth2/token";

/** Generate a P-256 key pair. */
async function generate(): Promise<CryptoKeyPair> {
    return crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, [
        "sign",
        "verify",
    ]);
}

test("authenticate a machine by its assertion once, refusing replayed, expired, forged and foreign ones", async () => {
    const storage = await TestDatabase.create("sqlite", [proofReplay], { isMigrated: true });
    onTestFinished(() => storage.close());

    // publish a machine's key, and verify assertions against it
    const pair = await generate();
    const forger = await generate();
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.machine.reference(Scope.universe.id, "machine-1"),
        publicKey: await PublicKey.of(pair.publicKey),
    };
    const verifier = new AssertionVerifier({
        endpoint,
        audience,
        keys: async (id) => (id === key.id ? key : undefined),
        database: storage.database,
    });
    const now = Date.now();
    const signed = await Assertion.sign(pair.privateKey, key, endpoint, now);

    // sign a late, forged, unknown, foreign and untyped assertion
    const issuedAt = Math.floor(now / 1000);
    const foreign = "https://other.example/oauth2/token";
    const untyped = new SignJWT({ jti: "untyped" })
        .setProtectedHeader({ alg: "ES256", kid: key.id })
        .setIssuer(key.subject.id)
        .setSubject(key.subject.id)
        .setAudience(endpoint)
        .setIssuedAt(issuedAt)
        .setExpirationTime(issuedAt + 60);
    const refused: [string, number][] = [
        [signed, now],
        [await Assertion.sign(pair.privateKey, key, endpoint, now), now + 120_000],
        [await Assertion.sign(forger.privateKey, key, endpoint, now), now],
        [await Assertion.sign(pair.privateKey, { ...key, id: "key-2" }, endpoint, now), now],
        [await Assertion.sign(pair.privateKey, key, foreign, now), now],
        [await untyped.sign(pair.privateKey), now],
    ];

    // verify the first once, then replay it and verify the others
    const caller = await verifier.verify(signed, now);
    const refusals = [];
    for (const [token, at] of refused) {
        refusals.push(await refusal(verifier.verify(token, at)));
    }

    // answer the machine as the caller, and refuse every other assertion
    expect({ caller, refusals }).toEqual({
        caller: {
            subject: key.subject,
            subjects: [key.subject],
            assurance: { level: KEY_ASSURANCE, authenticatedAt: now },
            credential: {
                kind: MACHINE_KEY,
                id: key.id,
                audience,
                verifiedAt: now,
                expiresAt: (Math.floor(now / 1000) + 60) * 1000,
            },
        },
        refusals: [
            ["UNAUTHORIZED", "assertion was used before"],
            ["UNAUTHORIZED", "assertion expired"],
            ["UNAUTHORIZED", "invalid assertion"],
            ["UNAUTHORIZED", "assertion has no standing key"],
            ["UNAUTHORIZED", "invalid assertion"],
            ["UNAUTHORIZED", "invalid assertion"],
        ],
    });
});

test("authenticate an installation within its space", async () => {
    const storage = await TestDatabase.create("sqlite", [proofReplay], { isMigrated: true });
    onTestFinished(() => storage.close());

    // publish an installation's key and verify its assertion
    const pair = await generate();
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.installation.reference("space-1", "installation-1"),
        publicKey: await PublicKey.of(pair.publicKey),
    };
    const verifier = new AssertionVerifier({
        endpoint,
        audience,
        keys: async () => key,
        database: storage.database,
    });
    const now = Date.now();
    const caller = await verifier.verify(
        await Assertion.sign(pair.privateKey, key, endpoint, now),
        now,
    );

    // bind the installation's credential to its space
    expect([caller.subject, caller.credential.kind, caller.credential.scope]).toEqual([
        key.subject,
        "installation-key",
        "space-1",
    ]);
});

test("authenticate a space and its installation by the space's key, refusing a subject the key cannot assert", async () => {
    const storage = await TestDatabase.create("sqlite", [proofReplay], { isMigrated: true });
    onTestFinished(() => storage.close());

    // publish a space's key, found under the issuer its assertions claim
    const pair = await generate();
    const spaceId = Identifier.create("space");
    const key: AssertionKey = {
        id: "key-1",
        subject: principal.space.reference(Scope.universe.id, spaceId),
        publicKey: await PublicKey.of(pair.publicKey),
    };
    const verifier = new AssertionVerifier({
        endpoint,
        audience,
        keys: async (id, issuer) => (id === key.id && issuer === spaceId ? key : undefined),
        database: storage.database,
    });

    // assert an installation of the space, the space itself, and a machine
    const now = Date.now();
    const installationId = Identifier.create("installation");
    const sign = (subject: string) => Assertion.sign(pair.privateKey, key, endpoint, now, subject);
    const caller = await verifier.verify(await sign(installationId), now);
    const itself = await verifier.verify(await sign(spaceId), now);
    const refused = await refusal(verifier.verify(await sign(Identifier.create("machine")), now));

    // bind the installation to the space and let the space act as itself, refusing what its key cannot assert
    expect({
        installation: [caller.subject, caller.credential.kind, caller.credential.scope],
        space: [itself.subject, itself.credential.kind, itself.credential.scope],
        refused,
    }).toEqual({
        installation: [
            principal.installation.reference(spaceId, installationId),
            "installation-key",
            spaceId,
        ],
        space: [principal.space.reference(Scope.universe.id, spaceId), "space-key", undefined],
        refused: ["UNAUTHORIZED", "assertion subject is outside its key's authority"],
    });
});
