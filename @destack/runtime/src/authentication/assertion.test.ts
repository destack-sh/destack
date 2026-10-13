import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { KEY_ASSURANCE, principal } from "@destack/access";
import { PublicKey } from "@destack/identity";
import { PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { SignJWT } from "jose";
import { Assertion, AssertionVerifier, proofReplay, type AssertionKey } from "./assertion.ts";
import { MACHINE_KEY } from "./token.ts";

/** The package receiving the calls. */
const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000031");

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
        audience,
        keys: async (id) => (id === key.id ? key : undefined),
        database: storage.database,
    });
    const now = Date.now();
    const signed = await Assertion.sign(pair.privateKey, key, audience, now);

    // sign a late, forged, unknown, foreign and untyped assertion
    const issuedAt = Math.floor(now / 1000);
    const foreign = PackageId.parse("package-019f7480-0000-7000-8000-000000000032");
    const untyped = new SignJWT({ jti: "untyped" })
        .setProtectedHeader({ alg: "ES256", kid: key.id })
        .setIssuer(key.subject.id)
        .setSubject(key.subject.id)
        .setAudience(audience)
        .setIssuedAt(issuedAt)
        .setExpirationTime(issuedAt + 60);
    const refused: [string, number][] = [
        [signed, now],
        [await Assertion.sign(pair.privateKey, key, audience, now), now + 120_000],
        [await Assertion.sign(forger.privateKey, key, audience, now), now],
        [await Assertion.sign(pair.privateKey, { ...key, id: "key-2" }, audience, now), now],
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
        audience,
        keys: async () => key,
        database: storage.database,
    });
    const now = Date.now();
    const caller = await verifier.verify(
        await Assertion.sign(pair.privateKey, key, audience, now),
        now,
    );

    // bind the installation's credential to its space
    expect([caller.subject, caller.credential.kind, caller.credential.scope]).toEqual([
        key.subject,
        "installation-key",
        "space-1",
    ]);
});
