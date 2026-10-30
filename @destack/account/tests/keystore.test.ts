import type { Environment } from "@destack/environment";
import { SpaceMount } from "@destack/object";
import { Jwks, Jws, Jwt, LocalKeyring, MemoryKeychain, Recipient } from "@destack/identity";
import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import { SPACE_KEY, TokenVerifier } from "@destack/access/token";
import { principal, universe } from "@destack/access";
import { Scope } from "@destack/sync";
import accountPackage from "../src/package.ts";
import { expect, onTestFinished, refusal, single, test } from "@destack/test";
import {
    identityKey,
    IdentityKeystore,
    IdentityRegistry,
    KEY_SET_MILLISECONDS,
} from "../src/identity/index.ts";
import { Identity } from "../src/object/index.ts";
import { TestAccountStore, OWNER_EMAIL } from "../src/test/index.ts";
import { accountDatabase } from "../src/stack/index.ts";
import { TestDatabase } from "@destack/db/test";
import { RequestId } from "@destack/sync";

/** Read the key of a token. */
function kid(token: string): string | undefined {
    return Jws.header(token).kid;
}

/** Serve a person's spaces on a machine, with a keystore of the spaces' identities. */
async function openKeystore() {
    // place a person's space on a machine they enroll, which starts and extends the identities of what it serves
    const accounts = await TestAccountStore.open();
    onTestFinished(() => accounts[Symbol.asyncDispose]());
    const { clock, environment } = accounts;
    const owner = await accounts.signIn(OWNER_EMAIL);
    const { accountId, machine } = await accounts.createSpace(owner);
    const registry = IdentityRegistry.client(machine.client(accountPackage), accounts.environment);

    // place each further space on the same machine
    const place = async (): Promise<Identifier<"space">> => {
        const spaceId = Identifier.create("space", accounts.environment);
        await owner.client(accountPackage).space.create({
            accountId,
            id: spaceId,
            requestId: RequestId.create(environment),
            name: `space-${spaceId.slice(-12)}`,
            machine: schema.identifier("machine").parse(machine.subject.id),
        });

        return spaceId;
    };
    const keystore = new IdentityKeystore(
        await LocalKeyring.read(LocalKeyring.generate(accounts.environment), accounts.environment),
        registry,
        accounts.environment,
    );

    return {
        accounts,
        clock,
        simulation: accounts.simulation,
        database: accounts.database,
        registry,
        place,
        keystore,
    };
}

/** Open an account database holding a copy of a space's keys, as a move's other machine does. */
async function openKeys(environment: Environment): Promise<TestDatabase> {
    const storage = await TestDatabase.create(accountDatabase, { environment, isMigrated: true });
    onTestFinished(() => storage.close());

    return storage;
}

/** Verify a token by the key set of what a space signs, answering its subject. */
async function verifiedSubject(
    token: string,
    database: DatabaseConnection,
    space: string,
    environment: Pick<Environment, "clock" | "crypto">,
): Promise<string | undefined> {
    const { crypto } = environment;
    const keys = Jwks.local(await Identity.keys(Snapshot.live(database), space, crypto), crypto);
    const options = { algorithms: ["ES256"], now: environment.clock.now() } as const;

    return (await Jwt.verify(token, keys, options, crypto)).claims.sub;
}

/** Derive 32 bytes under a label from a space's root secret in a keystore, as hexadecimal. */
async function derive(
    keystore: IdentityKeystore,
    database: DatabaseConnection,
    space: Identifier<"space">,
    label: string,
): Promise<string> {
    const root = present(await keystore.deriver(database, space), "the space's root");

    return (await root.derive({ label, context: "" })).toHex();
}

test("sign with the active key until every verifier read the next one, verifying tokens of every published key with a key set a verifier caches", async () => {
    // start a space's identity on a clock the test moves
    const { clock, simulation, database, place, keystore, registry } = await openKeystore();
    const subject = await place();
    await keystore.generate(database, subject);
    const sign = () => keystore.sign(database, subject, { sub: "user-1" });
    const verified = (token: string) =>
        verifiedSubject(token, database, subject, keystore.environment);

    // sign and verify with the first key, then rotate to a next key
    const first = await sign();
    await verified(first);
    await keystore.rotate(database, subject);

    // sign with the first key until the verifiers' key set age passed, then with the next one
    const during = await sign();
    await simulation.jump(clock.now() + KEY_SET_MILLISECONDS);
    const after = await sign();

    // sign with the first published key until the next, and verify every token with the key set read again once it aged
    const { keys } = await Identity.keys(
        Snapshot.live(database),
        subject,
        keystore.environment.crypto,
    );
    const firstKey = kid(first);
    const nextKey = single(keys.filter((key) => key.kid !== firstKey)).kid;
    const published = present(await registry.find(subject), "the published identity");
    expect([
        [kid(first), kid(during), kid(after)],
        await verified(first),
        await verified(after),
        published.signingKeys.length,
    ]).toEqual([[firstKey, firstKey, nextKey], "user-1", "user-1", 2]);
});

test("rekey an identity once a move completes, so the keys its source stored neither sign for it nor rotate it", async () => {
    // start a space's identity, storing a replica of its keys as a move's source stores them
    const { accounts, database, place, keystore, registry } = await openKeystore();
    const source = await openKeys(accounts.environment);
    const subject = await place();
    await keystore.generate(database, subject);
    const keys = await database.select().from(identityKey);
    await source.database.insert(identityKey).values(keys);
    const moved = new IdentityKeystore(keystore.keyring, registry, keystore.environment);
    const before = await moved.sign(source.database, subject, { sub: "user-1" });

    // rekey on the new machine, and sign and rotate with the source's keys
    await keystore.rekey(database, subject);
    const after = await keystore.sign(database, subject, { sub: "user-1" });
    const verified = (token: string) =>
        verifiedSubject(token, database, subject, keystore.environment);

    // verify the fresh key alone, and refuse the source's rotation
    expect([
        await refusal(verified(before)),
        await verified(after),
        await refusal(moved.rotate(source.database, subject)),
    ]).toEqual([
        ["UNAUTHORIZED", "KEY_NOT_FOUND", "no key of the set signed the token"],
        "user-1",
        [
            "UNAUTHORIZED",
            "OPERATION_UNSIGNED",
            "the operation is not signed by a rotation key of the identity",
        ],
    ]);
});

test("sign an installation's request for the space it goes to by its space's key, and refuse a request to no space", async () => {
    // start a space's identity on its machine
    const { accounts, database, place, keystore } = await openKeystore();
    const space = await place();
    await keystore.generate(database, space);

    // send a request as an installation of the space
    const audience = PackageId.parse("package-01996ab0-0000-7000-8000-000000000004");
    const installation = principal.installation.reference(
        space,
        "installation-01996ab0-0000-7000-8000-000000000005",
    );
    let received: Request | undefined;
    const send = keystore
        .source(database, installation, {
            environment: accounts.environment,
            issuer: "https://universe.test/oauth",
            send: async (request) => {
                received = request;

                return new Response(null, { status: 204 });
            },
        })
        .fetchSigned(audience);
    const target = "space-01996ab0-0000-7000-8000-0000000000aa";
    await send(new Request(SpaceMount.url("https://machine.test/", target)));
    const unscoped = await refusal(send(new Request("https://notes.test/")));

    // verify what arrives by the space's keys, as a receiver in another space does
    const verifier = new TokenVerifier({
        authority: { kind: "space", space },
        issuer: space,
        audience,
        keys: await Identity.keys(Snapshot.live(database), space, keystore.environment.crypto),
        environment: accounts.environment,
    });
    const verified = await verifier.authenticate(
        present(received, "the sent request"),
        keystore.environment.clock.now(),
    );
    expect([
        verified.caller.subject,
        verified.credential.kind,
        verified.credential.scope,
        verified.credential.deployments,
        unscoped,
    ]).toEqual([
        installation,
        SPACE_KEY,
        target,
        undefined,
        [
            "BAD_REQUEST",
            "SPACE_REQUIRED",
            `${installation.id} can only sign calls to spaces, not https://notes.test/`,
        ],
    ]);
});

test("derive one key per space and label, the same after a move rewraps the space's secret", async () => {
    // store two spaces' keys under the source machine's keyring
    const opened = await openKeystore();
    const world = opened.accounts.environment;
    const source = opened.database;
    const targetStorage = await openKeys(world);
    const target = targetStorage.database;
    const sourceKeystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), "source", world),
        opened.registry,
        opened.accounts.environment,
    );
    const targetKeystore = new IdentityKeystore(
        await LocalKeyring.open(new MemoryKeychain(), "target", world),
        opened.registry,
        opened.accounts.environment,
    );
    const spaces = [await opened.place(), await opened.place()] as const;
    for (const space of spaces) {
        await sourceKeystore.generate(source, space);
    }

    // move the first space's keys: encrypt them to the target and open them under its keyring
    const recipient = await Recipient.generate(world);
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

    // expect one key per space and label across derivations and the move
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
    // take the universe's identity, which its start generates
    await using accounts = await TestAccountStore.open();
    const { clock, database, keystore } = accounts;

    // call a package in a space as the universe
    const issuer = "https://universe.test/package-01996ab0-0000-7000-8000-0000000000a1/oauth";
    const audience = PackageId.parse("package-01996ab0-0000-7000-8000-000000000012");
    const space = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");
    const received: Request[] = [];
    const universeFetch = keystore
        .source(database, universe.reference(Scope.universe.id, Scope.universe.id), {
            environment: accounts.environment,
            issuer,
            send: async (request) => {
                received.push(request);

                return new Response(null, { status: 204 });
            },
        })
        .fetchSigned(audience);
    await universeFetch(new Request(`https://machine.test/spaces/${space}/package-x/objects`));

    // verify it as the universe in that space, refuse it in another, and refuse a request to no space
    const verify = (spaceId: Identifier<"space">) =>
        new TokenVerifier({
            authority: { kind: "universe" },
            issuer,
            audience,
            keys: async (header) =>
                Jwks.local(
                    await Identity.keys(
                        Snapshot.live(database),
                        Scope.universe.id,
                        accounts.environment.crypto,
                    ),
                    accounts.environment.crypto,
                )(header),
            environment: accounts.environment,
        }).authenticate(present(received[0], "the sent request").clone(), clock.now(), spaceId);
    const verified = await verify(space);
    expect({
        verified: [verified.caller.subject, verified.credential.kind, verified.credential.scope],
        elsewhere: await refusal(
            verify(schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000013")),
        ),
        spaceless: await refusal(universeFetch(new Request("https://machine.test/package-x"))),
    }).toEqual({
        verified: [universe.reference(Scope.universe.id, Scope.universe.id), "universe-key", space],
        elsewhere: ["UNAUTHORIZED", "TOKEN_INVALID", "invalid access token claims"],
        spaceless: [
            "BAD_REQUEST",
            "SPACE_REQUIRED",
            `${Scope.universe.id} can only sign calls to spaces, not https://machine.test/package-x`,
        ],
    });
});
