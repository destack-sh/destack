import { LocalKeyring, MemoryKeychain, Recipient } from "@destack/identity";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import { SPACE_KEY, TokenVerifier } from "@destack/runtime/authentication";
import { principal, universe } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { ObjectStore } from "@destack/object/server";
import { testEvents, serversOf } from "@destack/object/test";
import { Package } from "@destack/package";
import { testCallKey } from "@destack/runtime/test";
import { Scope } from "@destack/sync";
import accountPackage from "../src/package.ts";
import { implementIdentity, implementOperation } from "../src/server/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { createLocalJWKSet, decodeProtectedHeader, jwtVerify } from "jose";
import {
    identityKey,
    IdentityKeystore,
    IdentityRegistry,
    KEY_SET_MILLISECONDS,
} from "../src/identity/index.ts";
import { identity, Identity, placement } from "../src/object/index.ts";
import { AccountFixture, outcome, TestIdentityRegistry } from "../src/test/index.ts";
import type { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** Read the key of a token. */
function kid(token: string): string | undefined {
    return decodeProtectedHeader(token).kid;
}

/** Open an account database with a machine serving spaces and a keystore of their identities. */
async function openKeystore(clock: Clock) {
    const storage = await AccountFixture.storage(undefined, "sqlite");
    onTestFinished(() => storage.close());
    const { registry, place } = await TestIdentityRegistry.open(
        storage.database,
        clock,
        Identifier.create("machine"),
    );
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate()),
        registry,
        clock,
    );

    return { database: storage.database, registry, place, keystore };
}

/** Read the key set verifying what a space signs, as a verifier reads it now. */
async function keySet(database: DatabaseConnection, space: string) {
    return createLocalJWKSet(await Identity.keys(Snapshot.live(database), space));
}

test("sign with the active key until every verifier read the next one, verifying tokens of every published key with a key set a verifier keeps", async () => {
    // start a space's identity on a clock the test moves
    const clock = new VirtualClock(Date.now());
    const { database, place, keystore, registry } = await openKeystore(clock);
    const subject = await place();
    await keystore.generate(database, subject);
    const sign = () => keystore.sign(database, subject, { sub: "user-1" });
    const verified = async (token: string) =>
        (await jwtVerify(token, await keySet(database, subject))).payload.sub;

    // sign and verify with the first key, then rotate to a next key
    const first = await sign();
    await verified(first);
    await keystore.rotate(database, subject);

    // keep signing with the first key until the verifiers' key set age passed, then with the next one
    const during = await sign();
    clock.advance(KEY_SET_MILLISECONDS);
    const after = await sign();

    // verify every token with the key set the verifier read again once it aged
    expect([
        kid(during) === kid(first),
        kid(after) === kid(first),
        await verified(first),
        await verified(after),
        (await registry.find(subject))?.signingKeys.length,
    ]).toEqual([true, false, "user-1", "user-1", 2]);
});

test("rekey an identity once a move completes, so the keys its source kept neither sign for it nor rotate it", async () => {
    // start a space's identity, keeping a replica of its keys as a move's source keeps them
    const { database, place, keystore, registry } = await openKeystore(
        new VirtualClock(Date.now()),
    );
    const source = await AccountFixture.storage(undefined, "sqlite");
    onTestFinished(() => source.close());
    const subject = await place();
    await keystore.generate(database, subject);
    const kept = await database.select().from(identityKey);
    await source.database.insert(identityKey).values(kept);
    const moved = new IdentityKeystore(keystore.keyring, registry, keystore.clock);
    const before = await moved.sign(source.database, subject, { sub: "user-1" });

    // rekey on the new machine, and sign and rotate with the source's keys
    await keystore.rekey(database, subject);
    const after = await keystore.sign(database, subject, { sub: "user-1" });
    const verified = async (token: string) =>
        (await jwtVerify(token, await keySet(database, subject))).payload.sub;

    // verify the fresh key alone, and refuse the source's rotation
    expect([
        await verified(after),
        await outcome(verified(before)),
        await outcome(moved.rotate(source.database, subject)),
    ]).toEqual([
        "user-1",
        "ERR_JWKS_NO_MATCHING_KEY: no applicable key found in the JSON Web Key Set",
        "UNAUTHORIZED: no rotation key of the identity signed the operation",
    ]);
});

test("send a request as an installation with a token its space's key signs, which a verifier of the space's keys accepts", async () => {
    // start a space's identity on its machine
    const { database, place, keystore } = await openKeystore(new VirtualClock(Date.now()));
    const space = await place();
    await keystore.generate(database, space);

    // send a request as an installation of the space
    const audience = PackageId.parse("package-01996ab0-0000-7000-8000-000000000004");
    const installation = principal.installation.reference(
        space,
        "installation-01996ab0-0000-7000-8000-000000000005",
    );
    let received: Request | undefined;
    const send = keystore.fetch(database, installation, audience, async (request) => {
        received = request;

        return new Response(null, { status: 204 });
    });
    await send(new Request("https://notes.test/"));

    // verify what arrives by the space's keys, as a receiver in another space does
    const verifier = new TokenVerifier({
        authority: { kind: "space", space },
        issuer: space,
        audience,
        keys: await Identity.keys(Snapshot.live(database), space),
    });
    const verified = await verifier.authenticate(present(received, "the sent request"), Date.now());
    expect([
        verified.caller.subject,
        verified.credential.kind,
        verified.credential.deployments,
    ]).toEqual([installation, SPACE_KEY, undefined]);
});

/** Derive 32 bytes under a label from a space's root secret in a keystore, as hexadecimal. */
async function derive(
    keystore: IdentityKeystore,
    database: DatabaseConnection,
    space: Identifier<"space">,
    label: string,
): Promise<string> {
    const root = present(await keystore.deriver(database, space), "the space's root");

    return (await root.derive(label)).toHex();
}

test("derive one key per space and label, the same after a move rewraps the space's secret", async () => {
    // keep two spaces' keys under the source machine's keyring
    const clock = new VirtualClock(Date.now());
    const opened = await openKeystore(clock);
    const source = opened.database;
    const targetStorage = await AccountFixture.storage(undefined, "sqlite");
    onTestFinished(() => targetStorage.close());
    const target = targetStorage.database;
    const sourceKeystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), "source"),
        opened.registry,
        clock,
    );
    const targetKeystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), "target"),
        opened.registry,
        clock,
    );
    const spaces = [await opened.place(), await opened.place()] as const;
    for (const space of spaces) {
        await sourceKeystore.generate(source, space);
    }

    // move the first space's keys: encrypt them to the target and open them under its keyring
    const recipient = await Recipient.generate();
    const [row] = await source.select().from(identityKey).where(eq(identityKey.scope, spaces[0]));
    const sealed = await sourceKeystore.rewrap.seal(present(row, "the space's keys"), recipient);
    await target.insert(identityKey).values(await targetKeystore.rewrap.open(sealed, recipient));

    // derive under two labels in both spaces and on the target after the move
    const derived = [
        await derive(sourceKeystore, source, spaces[0], "first"),
        await derive(sourceKeystore, source, spaces[0], "first"),
        await derive(sourceKeystore, source, spaces[0], "second"),
        await derive(sourceKeystore, source, spaces[1], "first"),
        await derive(targetKeystore, target, spaces[0], "first"),
    ];

    // keep one key per space and label across derivations and the move
    const [first, , second, other] = derived;
    expect({
        derived,
        distinct: new Set(derived).size,
        absent: await targetKeystore.deriver(target, spaces[1]),
    }).toEqual({
        derived: [first, first, second, other, first],
        distinct: 3,
        absent: undefined,
    });
});

test("send a request as the universe bound to the space it goes to, which a verifier of the universe's keys accepts for that space alone", async () => {
    // start the universe's identity in its own registry
    const storage = await AccountFixture.storage(undefined, "sqlite");
    onTestFinished(() => storage.close());
    const { database } = storage;
    const clock = new VirtualClock(Date.now());
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate()),
        await universeRegistry(database, clock),
        clock,
    );
    await keystore.generate(database, Scope.universe.id);

    // call a package in a space as the universe
    const issuer = "https://universe.test/package-01996ab0-0000-7000-8000-0000000000a1/oauth";
    const audience = PackageId.parse("package-01996ab0-0000-7000-8000-000000000012");
    const space = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");
    const received: Request[] = [];
    const universeFetch = keystore.fetchAsUniverse(database, issuer, async (request) => {
        received.push(request);

        return new Response(null, { status: 204 });
    });
    await universeFetch(audience)(
        new Request(`https://machine.test/spaces/${space}/package-x/objects`),
    );

    // verify it as the universe in that space, refuse it in another, and refuse a request to no space
    const verify = (spaceId: string) =>
        new TokenVerifier({
            authority: { kind: "universe" },
            issuer,
            audience,
            keys: async (header, token) =>
                createLocalJWKSet(await Identity.keys(Snapshot.live(database), Scope.universe.id))(
                    header,
                    token,
                ),
        }).authenticate(present(received[0], "the sent request").clone(), clock.now(), spaceId);
    const verified = await verify(space);
    expect({
        verified: [verified.caller.subject, verified.credential.kind, verified.credential.scope],
        elsewhere: await outcome(verify("space-01996ab0-0000-7000-8000-000000000013")),
        spaceless: await universeFetch(audience)(
            new Request("https://machine.test/package-x"),
        ).catch((error: unknown) => (error instanceof TypeError ? error.message : error)),
    }).toEqual({
        verified: [universe.reference(Scope.universe.id, Scope.universe.id), "universe-key", space],
        elsewhere: "UNAUTHORIZED: invalid access token claims",
        spaceless:
            "the universe calls packages in spaces alone, not https://machine.test/package-x",
    });
});

/** Serve the universe's identity registry as the universe, which may start its own identity. */
async function universeRegistry(database: DatabaseConnection, clock: Clock) {
    // let the universe start and extend identities, as the platform role does
    const self = universe.reference(Scope.universe.id, Scope.universe.id);
    await new AccessFixture(database).replicateRole(
        Scope.universe,
        {
            name: "platform",
            description: "Start and extend the universe's identity",
            permissions: [identity.permission("start"), identity.permission("extend")],
        },
        self,
        clock.now(),
    );

    // serve the identities as the universe
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: Scope.universe,
        servers: serversOf([implementIdentity(), implementOperation()]),
        policies: [placement],
        database,
        callKey: testCallKey,
        events: await testEvents(database, [], clock),
        origin: { package: Package.of(accountPackage) },
    });

    return IdentityRegistry.of(store);
}
