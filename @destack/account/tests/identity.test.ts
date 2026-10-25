import { Scope, RequestId } from "@destack/sync";
import { principal } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import {
    type PartitionId,
    TOKEN_LIFETIME_MILLISECONDS,
    TokenVerifier,
} from "@destack/access/token";
import { expect, refusal, test } from "@destack/test";
import { found, Identifier, present, schema } from "@destack/schema";
import { MachineIdentity } from "../src/client/index.ts";
import { TOKEN_PATH } from "../src/authentication/path.ts";
import { Identity, key, machine } from "../src/object/index.ts";
import { JWT_BEARER } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { MemoryKeychain } from "@destack/identity";
import {
    TestAccountStore,
    TestJwt,
    type TestPerson,
    ids,
    ISSUER,
    OPERATOR_EMAIL,
    OWNER_EMAIL,
    STRANGER_EMAIL,
} from "../src/test/index.ts";
import { ANY_STRING, Tokens } from "./fixture.ts";

/** The token endpoint every assertion is for. */
const ENDPOINT = `${ISSUER}${TOKEN_PATH}`;

/** The refusal of an assertion without a standing key behind it. */
const NO_STANDING_KEY = [
    400,
    { error: "invalid_grant", error_description: "assertion has no standing key" },
];

/** The package receiving the machines' requests. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e002");

/** A stored private key's public coordinates, beside its private scalar. */
const StoredKey = schema.looseObject({
    kty: schema.string(),
    crv: schema.string(),
    x: schema.string(),
    y: schema.string(),
});

/** Ask the token endpoint for a token of the audience by an assertion, answering a refusal. */
async function ask(
    accounts: TestAccountStore,
    assertion: string,
    partition?: PartitionId,
): Promise<"done" | readonly [number, unknown]> {
    const response = await accounts.fetch(
        new Request(ENDPOINT, {
            method: "POST",
            body: new URLSearchParams({
                grant_type: JWT_BEARER,
                assertion,
                resource: PackageId.resource(AUDIENCE),
                ...(partition === undefined ? {} : { partition }),
            }),
        }),
    );
    const body: unknown = await response.json();

    // answer issued tokens as done, and a refusal as its status and body
    if (response.ok) {
        Tokens.parse(body);

        return "done";
    }

    return [response.status, body];
}

/** Grant a machine a token for the audience by a fresh assertion, in a partition when given. */
async function grant(
    accounts: TestAccountStore,
    signer: MachineIdentity,
    partition?: PartitionId,
): Promise<"done" | readonly [number, unknown]> {
    return ask(accounts, await signer.assert(ENDPOINT), partition);
}

/** Refuse a machine a token in a space other machines serve. */
function placedElsewhere(spaceId: Identifier<"space">) {
    return [
        400,
        { error: "invalid_grant", error_description: `${spaceId} is placed on other machines` },
    ];
}

/** Create a space in the owner's account, placed on a machine. */
async function place(
    accounts: TestAccountStore,
    owner: TestPerson,
    id: Identifier<"space">,
    machineId: Identifier<"machine">,
): Promise<void> {
    await owner.client(accountPackage).space.create({
        accountId: ids.account,
        id,
        requestId: RequestId.create(accounts.environment),
        name: `space-${id.slice(-12)}`,
        machine: machineId,
    });
}

/** Enroll a cloud machine of the platform as the operator, its first key stored apart. */
async function enrollCloud(accounts: TestAccountStore) {
    // enroll the machine with a keychain of its own
    const operator = await accounts.signIn(OPERATOR_EMAIL);
    const keys = new MemoryKeychain();
    const identity = new MachineIdentity(
        Identifier.create("machine", accounts.environment),
        keys,
        accounts.environment,
    );
    await identity.enroll(operator.client(accountPackage), {
        scope: accounts.platform,
        requestId: RequestId.create(accounts.environment),
        name: "edge",
        kind: "cloud",
        residencyId: "eu",
        providerCode: "cloudflare",
        location: "eu",
    });
    accounts.register(identity);

    // copy the first key to a keychain the machine's rotations leave alone
    const keychain = new MemoryKeychain();
    keychain.secrets.set(identity.machineId, found(keys.secrets, identity.machineId));
    const first = new MachineIdentity(identity.machineId, keychain, accounts.environment);
    const enrolled = present(
        await Snapshot.live(accounts.database)
            .read(key.table)
            .findFirst({ where: { parentId: identity.machineId } }),
        "the enrolled machine's key",
    );

    return { operator, identity, first, enrolled };
}

test("enroll a machine as its account's owner, authenticate it across a key rotation, and refuse its tokens once revoked", async () => {
    // enroll a machine under the owner's account and read itself by its token
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const stranger = await accounts.signIn(STRANGER_EMAIL);
    const accountId = ids.account;
    const identity = await accounts.enroll(accountId, owner);
    const itself = () =>
        refusal(
            accounts
                .as(identity)
                .client(accountPackage)
                .machine.get({ scope: accountId, id: identity.machineId }),
        );
    expect(await itself()).toBe("done");

    // refuse a stranger enrolling a machine into the account and a proof for another machine
    const stray = new MachineIdentity(
        Identifier.create("machine", accounts.environment),
        new MemoryKeychain(),
        accounts.environment,
    );
    const enrolling = {
        scope: accountId,
        requestId: RequestId.create(accounts.environment),
        name: "stray",
        kind: "personal" as const,
        platform: "macOS" as const,
        architecture: "arm64",
        model: "Mac14,2",
    };
    const other = new MachineIdentity(
        Identifier.create("machine", accounts.environment),
        new MemoryKeychain(),
        accounts.environment,
    );
    const { publicKey } = await other.generate();
    expect([
        await refusal(stray.enroll(stranger.client(accountPackage), enrolling)),
        await refusal(
            owner.client(accountPackage).machine.enroll({
                ...enrolling,
                id: Identifier.create("machine", accounts.environment),
                publicKey,
                proof: await other.prove(),
            }),
        ),
    ]).toEqual([
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`],
        ["BAD_REQUEST", "PROOF_INVALID", "key proof is for another subject or time"],
    ]);

    // number a taken name at enrollment and refuse taken names and branch separators
    const namesake = new MachineIdentity(
        Identifier.create("machine", accounts.environment),
        new MemoryKeychain(),
        accounts.environment,
    );
    const read = () =>
        accounts
            .as(identity)
            .client(accountPackage)
            .machine.get({ scope: accountId, id: identity.machineId });
    const taken = (await read()).name;
    const rename = (name: string) =>
        refusal(
            accounts
                .as(identity)
                .client(accountPackage)
                .machine.rename({
                    scope: accountId,
                    id: identity.machineId,
                    requestId: RequestId.create(accounts.environment),
                    name,
                }),
        );
    expect([
        await refusal(
            namesake.enroll(owner.client(accountPackage), {
                ...enrolling,
                requestId: RequestId.create(accounts.environment),
                name: taken,
            }),
        ),
        await rename(`${taken}-2`),
        await rename("studio"),
        await rename("studio--main"),
        await refusal(
            owner.client(accountPackage).machine.rename({
                scope: accountId,
                id: identity.machineId,
                requestId: RequestId.create(accounts.environment),
                name: "atelier",
            }),
        ),
        await rename("studio"),
        (await read()).name,
    ]).toEqual([
        "done",
        ["CONFLICT", "DUPLICATE", "another machine has the same scope and name"],
        "done",
        [
            "BAD_REQUEST",
            "INPUT_INVALID",
            "invalid input: name: labels are lowercase letters, digits and single inner hyphens",
        ],
        "done",
        "done",
        "studio",
    ]);

    // rotate the machine's key as the machine and refuse the owner registering a key for it
    await identity.rotate(accounts.as(identity).client(accountPackage), accountId);
    expect(await itself()).toBe("done");
    const impostor = new MachineIdentity(
        identity.machineId,
        new MemoryKeychain(),
        accounts.environment,
    );
    const { publicKey: foreign, proof } = await impostor.generate();
    expect(
        await refusal(
            owner.client(accountPackage).key.create({
                scope: accountId,
                requestId: RequestId.create(accounts.environment),
                parent: { ...machine.typeReference, id: identity.machineId },
                publicKey: foreign,
                proof,
            }),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: rotate"]);

    // revoke the machine as the owner
    await owner.client(accountPackage).machine.revoke({
        scope: accountId,
        id: identity.machineId,
        requestId: RequestId.create(accounts.environment),
    });
    expect(await grant(accounts, identity)).toEqual(NO_STANDING_KEY);
});

test("grant a cloud machine a token for a minute as itself alone, by its enrolled key", async () => {
    // enroll a cloud machine of the residency and take a token as it
    await using accounts = await TestAccountStore.open();
    const { identity, enrolled } = await enrollCloud(accounts);
    const fetch = (request: Request) => accounts.fetch(request);
    const accessToken = await identity.token(AUDIENCE, { url: ISSUER }, fetch);

    // verify the token as the machine alone, by its key's thumbprint
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: ISSUER,
        audience: AUDIENCE,
        keys: await Identity.keys(
            Snapshot.live(accounts.database),
            Scope.universe.id,
            accounts.environment.crypto,
        ),
        environment: accounts.environment,
    });
    const verified = await verifier.authenticate(
        new Request("https://edge.test/spaces", {
            headers: { authorization: `Bearer ${accessToken}` },
        }),
        accounts.clock.now(),
    );
    const { credential } = verified;
    expect([
        { kind: credential.kind, id: credential.id },
        verified.caller.subject,
        verified.caller.groups,
        credential.expiresAt - credential.verifiedAt,
    ]).toEqual([
        { kind: "machine-key", id: enrolled.thumbprint },
        principal.machine.reference(Scope.universe.id, identity.machineId),
        [],
        TOKEN_LIFETIME_MILLISECONDS,
    ]);
});

test("spend each of a machine's assertions once, and refuse an assertion for another endpoint", async () => {
    // assert twice for the token endpoint and once for another
    await using accounts = await TestAccountStore.open();
    const { identity } = await enrollCloud(accounts);
    const assertion = await identity.assert(ENDPOINT);
    const other = await identity.assert("https://elsewhere.test/oauth2/token");

    // grant the first use alone
    expect([
        await ask(accounts, assertion),
        await ask(accounts, assertion),
        await ask(accounts, other),
    ]).toEqual([
        "done",
        [400, { error: "invalid_grant", error_description: "assertion was already used" }],
        [400, { error: "invalid_grant", error_description: "invalid assertion" }],
    ]);
});

test("refuse a machine's assertions by its first key once it rotates, and revoking the rotated key again", async () => {
    // rotate the cloud machine's key as the machine
    await using accounts = await TestAccountStore.open();
    const { identity, first, enrolled } = await enrollCloud(accounts);
    const accountId = accounts.platform;
    await identity.rotate(accounts.as(identity).client(accountPackage), accountId);

    // refuse revoking the first key again, and its assertions, and grant the new key's
    const revoked = await refusal(
        accounts
            .as(identity)
            .client(accountPackage)
            .key.revoke({
                scope: accountId,
                id: enrolled.id,
                requestId: RequestId.create(accounts.environment),
            }),
    );
    expect([revoked, await grant(accounts, first), await grant(accounts, identity)]).toEqual([
        ["CONFLICT", "CREDENTIAL_REVOKED", "key is revoked"],
        NO_STANDING_KEY,
        "done",
    ]);
});

test("refuse a cloud machine's grants while the operator disables it and once the operator revokes it", async () => {
    // disable the cloud machine, grant once enabled again, then revoke it
    await using accounts = await TestAccountStore.open();
    const { operator, identity } = await enrollCloud(accounts);
    const target = () => ({
        scope: accounts.platform,
        id: identity.machineId,
        requestId: RequestId.create(accounts.environment),
    });
    await operator.client(accountPackage).machine.disable(target());
    const disabled = await grant(accounts, identity);
    await operator.client(accountPackage).machine.enable(target());
    const enabled = await grant(accounts, identity);
    await operator.client(accountPackage).machine.revoke(target());

    // refuse every grant but the enabled one
    expect([disabled, enabled, await grant(accounts, identity)]).toEqual([
        NO_STANDING_KEY,
        "done",
        NO_STANDING_KEY,
    ]);
});

test("issue a machine's token at the store clock's time, verifiable after the clock advances to it", async () => {
    // run the account package ten minutes ahead of real time, standing still there
    await using fixture = await TestAccountStore.open();
    fixture.clock.advance(10 * 60_000);
    fixture.clock.pause();

    // declare a cloud machine on the same clock and take its token
    const identity = await fixture.declareCloudMachine(fixture.platform, "eu");
    const fetch = (request: Request) => fixture.fetch(request);
    const accessToken = await identity.token(AUDIENCE, { url: ISSUER }, fetch);

    // verify it at the clock's time, with exactly the provider's claims
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: ISSUER,
        audience: AUDIENCE,
        keys: await Identity.keys(
            Snapshot.live(fixture.database),
            Scope.universe.id,
            fixture.environment.crypto,
        ),
        environment: fixture.environment,
    });
    const verified = await verifier.authenticate(
        new Request("https://edge.test/spaces", {
            headers: { authorization: `Bearer ${accessToken}` },
        }),
        fixture.clock.now(),
    );
    const declared = present(
        await Snapshot.live(fixture.database)
            .read(key.table)
            .findFirst({ where: { parentId: identity.machineId } }),
        "the declared machine's key",
    );
    const issuedAt = Math.floor(fixture.clock.now() / 1000);
    const subject = principal.machine.reference(Scope.universe.id, identity.machineId);
    expect([verified.caller.subject, TestJwt.claims(accessToken)]).toEqual([
        subject,
        {
            sub: identity.machineId,
            aud: PackageId.resource(AUDIENCE),
            client_id: identity.machineId,
            azp: identity.machineId,
            scope: "",
            iss: ISSUER,
            iat: issuedAt,
            exp: issuedAt + TOKEN_LIFETIME_MILLISECONDS / 1000,
            jti: ANY_STRING,
            caller: {
                subject,
                groups: [],
                contacts: [],
                credential: {
                    kind: "machine-key",
                    id: declared.thumbprint,
                    assurance: { level: 3, authenticatedAt: fixture.clock.now() },
                },
            },
        },
    ]);
});

test("reuse a machine's token for several calls to one package, exchanging it once", async () => {
    // count the machine's exchanges at the token endpoint
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const identity = await accounts.enroll(ids.account, owner);
    const exchanged: string[] = [];
    const fetch = (request: Request) => {
        // record each request to the token endpoint
        if (new URL(request.url).pathname.endsWith(TOKEN_PATH)) {
            exchanged.push(request.url);
        }

        return accounts.fetch(request);
    };

    // read the token for three calls at once and one later, and one for another package
    const tokens = await Promise.all(
        [1, 2, 3].map(() => identity.token(AUDIENCE, { url: ISSUER }, fetch)),
    );
    const later = await identity.token(AUDIENCE, { url: ISSUER }, fetch);
    const other = await identity.token(accountPackage.id, { url: ISSUER }, fetch);

    // send the first token every time for the package, and another one for the other package
    expect({
        tokens,
        audiences: [TestJwt.claims(later).aud, TestJwt.claims(other).aud],
        exchanged,
    }).toEqual({
        tokens: [later, later, later],
        audiences: [PackageId.resource(AUDIENCE), PackageId.resource(accountPackage.id)],
        exchanged: [ENDPOINT, ENDPOINT],
    });
});

test("grant a machine tokens for its own partition and the spaces it serves, receives or works in, and refuse others", async () => {
    // enroll an account's machine, take the residency's shared cloud machine and place a space on each
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const laptop = await accounts.enroll(ids.account, owner);
    const cloud = accounts.cloud(ids.machine);
    const movedId = Identifier.create("space", accounts.environment);
    const served = Identifier.create("space", accounts.environment);
    const elsewhere = Identifier.create("space", accounts.environment);
    await place(accounts, owner, movedId, laptop.machineId);
    await place(accounts, owner, served, cloud.machineId);

    // grant a token in the machine's own partition, and refuse another machine's
    const partitions = [
        await grant(accounts, laptop, laptop.machineId),
        await grant(accounts, laptop, cloud.machineId),
    ];

    // refuse tokens in the spaces other machines serve
    const refused = [
        await grant(accounts, laptop, served),
        await grant(accounts, cloud, elsewhere),
        await grant(accounts, cloud, movedId),
    ];

    // grant a token in a space giving the machine work, until the space withdraws it
    const work = {
        accountId: ids.account,
        id: served,
        epoch: 1,
        machine: laptop.machineId,
        requestId: RequestId.create(accounts.environment),
    };
    await accounts.as(cloud).client(accountPackage).placement.assign(work);
    const assigned = await grant(accounts, laptop, served);
    await accounts
        .as(cloud)
        .client(accountPackage)
        .placement.unassign({ ...work, requestId: RequestId.create(accounts.environment) });
    const withdrawn = await grant(accounts, laptop, served);

    // grant a token in a space moving to the machine
    await accounts
        .as(laptop)
        .client(accountPackage)
        .placement.move({
            accountId: ids.account,
            id: movedId,
            epoch: 1,
            target: cloud.machineId,
            transferId: Identifier.create("transfer", accounts.environment),
            requestId: RequestId.create(accounts.environment),
        });
    expect([
        await grant(accounts, laptop, movedId),
        await grant(accounts, cloud, served),
        refused,
        await grant(accounts, cloud, movedId),
        [assigned, withdrawn],
        partitions,
    ]).toEqual([
        "done",
        "done",
        [placedElsewhere(served), placedElsewhere(elsewhere), placedElsewhere(movedId)],
        "done",
        ["done", placedElsewhere(served)],
        [
            "done",
            [
                400,
                {
                    error: "invalid_grant",
                    error_description: `${laptop.machineId} acts in its own partition and its spaces alone`,
                },
            ],
        ],
    ]);
});

test("disable, drain and enable a machine, refusing its token grants while disabled", async () => {
    // enroll a machine of the owner's account
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const stranger = await accounts.signIn(STRANGER_EMAIL);
    const identity = await accounts.enroll(ids.account, owner);
    const change = (person: TestPerson, name: "disable" | "drain" | "enable") =>
        refusal(
            person.client(accountPackage).machine[name]({
                scope: ids.account,
                id: identity.machineId,
                requestId: RequestId.create(accounts.environment),
            }),
        );

    // refuse the machine's grants while disabled and grant them while draining and enabled
    const outcomes = [];
    for (const name of ["disable", "drain", "enable"] as const) {
        outcomes.push(await change(owner, name), await grant(accounts, identity));
    }
    expect(outcomes).toEqual(["done", NO_STANDING_KEY, "done", "done", "done", "done"]);

    // refuse a stranger changing the status and any change once the machine is revoked
    const refused = await change(stranger, "disable");
    await owner.client(accountPackage).machine.revoke({
        scope: ids.account,
        id: identity.machineId,
        requestId: RequestId.create(accounts.environment),
    });
    expect([refused, await change(owner, "enable")]).toEqual([
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${ids.account} not found`],
        ["CONFLICT", "CREDENTIAL_REVOKED", "machine is revoked"],
    ]);
});

test("store one key when two processes generate a machine's first key at once, both adopting the one stored first", async () => {
    // generate the first key from two identities sharing one keychain
    await using accounts = await TestAccountStore.open();
    const keys = new MemoryKeychain();
    const machineId = Identifier.create("machine", accounts.environment);
    const generated = await Promise.all([
        new MachineIdentity(machineId, keys, accounts.environment).generate(),
        new MachineIdentity(machineId, keys, accounts.environment).generate(),
    ]);

    // answer the stored key's public half to both
    const { kty, crv, x, y } = StoredKey.parse(JSON.parse(found(keys.secrets, machineId)));
    expect(generated.map((entry) => entry.publicKey)).toEqual([
        { kty, crv, x, y },
        { kty, crv, x, y },
    ]);
});

/** Order public keys by their x coordinate. */
function byX(left: { readonly x: string }, right: { readonly x: string }): number {
    return left.x.localeCompare(right.x);
}

test("rotate once for two identities of one machine rotating at once, ending on the key the universe has as current", async () => {
    // enroll a machine at a clock standing still
    await using accounts = await TestAccountStore.open();
    accounts.clock.pause();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const accountId = ids.account;
    const keys = new MemoryKeychain();
    const identity = new MachineIdentity(
        Identifier.create("machine", accounts.environment),
        keys,
        accounts.environment,
    );
    await identity.enroll(owner.client(accountPackage), {
        scope: accountId,
        requestId: RequestId.create(accounts.environment),
        name: "racer",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
    });
    accounts.register(identity);
    const enrolled = StoredKey.parse(JSON.parse(found(keys.secrets, identity.machineId)));

    // rotate its key from two of its identities at once
    const rotations = [
        new MachineIdentity(identity.machineId, keys, accounts.environment),
        new MachineIdentity(identity.machineId, keys, accounts.environment),
    ];
    await Promise.all(
        rotations.map((rotation) =>
            rotation.rotate(accounts.as(rotation).client(accountPackage), accountId),
        ),
    );

    // register one new key, revoking the enrolled one, and end on the key the keychain stores
    const registered = await accounts.database
        .select({ publicKey: key.table.publicKey, revokedAt: key.table.revokedAt })
        .from(key.table)
        .where(eq(key.table.parentId, identity.machineId))
        .orderBy(key.table.createdAt, key.table.id);
    const { kty, crv, x, y } = StoredKey.parse(JSON.parse(found(keys.secrets, identity.machineId)));
    const expected = [
        {
            publicKey: { kty: enrolled.kty, crv: enrolled.crv, x: enrolled.x, y: enrolled.y },
            revokedAt: accounts.clock.now(),
        },
        { publicKey: { kty, crv, x, y }, revokedAt: null },
    ];
    expect(registered.toSorted((left, right) => byX(left.publicKey, right.publicKey))).toEqual(
        expected.toSorted((left, right) => byX(left.publicKey, right.publicKey)),
    );
});

test("authenticate a machine's token in a space it serves while its key stands, and refuse it once the machine is revoked", async () => {
    // enroll a machine serving a space, and grant it a token there by its assertion
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(OWNER_EMAIL);
    const identity = await accounts.enroll(ids.account, owner);
    const spaceId = Identifier.create("space", accounts.environment);
    await place(accounts, owner, spaceId, identity.machineId);
    const response = await accounts.fetch(
        new Request(ENDPOINT, {
            method: "POST",
            body: new URLSearchParams({
                grant_type: JWT_BEARER,
                assertion: await identity.assert(ENDPOINT),
                resource: PackageId.resource(AUDIENCE),
                partition: spaceId,
            }),
        }),
    );
    const token = Tokens.parse(await response.json()).access_token;

    // authenticate the token in the space, before and after the owner revokes the machine
    const authenticate = () =>
        Identity.authenticate(
            Snapshot.live(accounts.database),
            new Request("https://machine.test/", { headers: { authorization: `Bearer ${token}` } }),
            {
                audience: AUDIENCE,
                universe: ISSUER,
                partition: spaceId,
                now: accounts.clock.now(),
                environment: accounts.environment,
            },
        );
    const standing = present(await authenticate(), "the standing machine's authentication");
    await owner.client(accountPackage).machine.revoke({
        scope: ids.account,
        id: identity.machineId,
        requestId: RequestId.create(accounts.environment),
    });

    expect({ standing: standing.caller.subject, revoked: await refusal(authenticate()) }).toEqual({
        standing: principal.machine.reference(Scope.universe.id, identity.machineId),
        revoked: ["UNAUTHORIZED", "KEY_REVOKED", "the machine token's key is no longer valid"],
    });
});
