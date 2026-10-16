import { Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/runtime/request";
import { TOKEN_LIFETIME_MILLISECONDS, TokenVerifier } from "@destack/runtime/authentication";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { decodeJwt } from "jose";
import { found, Identifier, schema } from "@destack/schema";
import { MachineIdentity } from "../src/client/index.ts";
import { TOKEN_PATH } from "../src/authentication/path.ts";
import { Identity, key, machine } from "../src/object/index.ts";
import { JWT_BEARER } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { MemoryKeychain } from "@destack/identity";
import { AccountFixture, ids, ISSUER, TestDirectory, outcome } from "../src/test/index.ts";
import { Tokens } from "./fixture.ts";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** The refusal of an assertion without a standing key behind it. */
const NO_STANDING_KEY = "invalid_grant: assertion has no standing key";

/** The token endpoint every assertion is for. */
const ENDPOINT = `${ISSUER}${TOKEN_PATH}`;

/** A token endpoint's refusal (RFC 6749 5.2). */
const Refusal = schema.looseObject({ error: schema.string(), error_description: schema.string() });

/** The package receiving the machines' requests. */
const AUDIENCE = PackageId.parse("package-019f7480-0000-7000-8000-00000000e002");

/** A stored private key's public coordinates, beside its private scalar. */
const StoredKey = schema.looseObject({
    kty: schema.string(),
    crv: schema.string(),
    x: schema.string(),
    y: schema.string(),
});

/** The account package every scenario enrolls its machines in. */
let accounts: AccountFixture;

beforeAll(async () => {
    accounts = await AccountFixture.open();
});

afterAll(async () => {
    await accounts[Symbol.asyncDispose]();
});

/** Ask the token endpoint for a token of the audience with an assertion, answering its refusal. */
async function ask(assertion: string, spaceId?: Identifier<"space">): Promise<string> {
    const response = await accounts.fetch(
        new Request(ENDPOINT, {
            method: "POST",
            body: new URLSearchParams({
                grant_type: JWT_BEARER,
                assertion,
                resource: PackageId.resource(AUDIENCE),
                ...(spaceId === undefined ? {} : { space: spaceId }),
            }),
        }),
    );
    // answer issued tokens, read to their end, or the refusal
    if (response.ok) {
        Tokens.parse(await response.json());

        return "executed";
    }
    const refused = Refusal.parse(await response.json());

    return `${refused.error}: ${refused.error_description}`;
}

/** Grant a machine a token for the audience by a fresh assertion, in a space when given. */
async function grant(signer: MachineIdentity, spaceId?: Identifier<"space">): Promise<string> {
    return ask(await signer.assert(ENDPOINT), spaceId);
}

test("enroll a machine as its account's owner, authenticate it across a key rotation, and refuse its tokens once revoked", async () => {
    const accountId = ids.account;

    // enroll a machine under the owner's account and read itself by its token
    const identity = await accounts.enroll(accountId);
    const itself = () =>
        outcome(
            accounts.machine(identity).machine.get({ scope: accountId, id: identity.machineId }),
        );
    expect(await itself()).toBe("executed");

    // refuse a stranger enrolling a machine into the account and a proof for another machine
    const stranger = new MachineIdentity(
        Identifier.create("machine"),
        new MemoryKeychain(),
        Clock.system,
    );
    const enrolling = {
        scope: accountId,
        requestId: RequestId.create(Date.now()),
        name: "stray",
        kind: "personal" as const,
        platform: "macOS" as const,
        architecture: "arm64",
        model: "Mac14,2",
    };
    const other = new MachineIdentity(
        Identifier.create("machine"),
        new MemoryKeychain(),
        Clock.system,
    );
    const { publicKey } = await other.generate();
    expect([
        await outcome(stranger.enroll(accounts.user(ids.stranger), enrolling)),
        await outcome(
            accounts.user(ids.owner).machine.enroll({
                ...enrolling,
                id: Identifier.create("machine"),
                publicKey,
                proof: await other.prove(),
            }),
        ),
    ]).toEqual([
        `NOT_FOUND: no scope ${accountId}`,
        "BAD_REQUEST: key proof is for another subject or time",
    ]);

    // number a taken name at enrollment and refuse taken names and branch separators
    const namesake = new MachineIdentity(
        Identifier.create("machine"),
        new MemoryKeychain(),
        Clock.system,
    );
    const taken = (
        await accounts.machine(identity).machine.get({ scope: accountId, id: identity.machineId })
    ).name;
    const rename = (name: string) =>
        outcome(
            accounts.machine(identity).machine.rename({
                scope: accountId,
                id: identity.machineId,
                requestId: RequestId.create(Date.now()),
                name,
            }),
        );
    expect([
        await outcome(
            namesake.enroll(accounts.user(ids.owner), {
                ...enrolling,
                requestId: RequestId.create(Date.now()),
                name: taken,
            }),
        ),
        await rename(`${taken}-2`),
        await rename("studio"),
        await rename("studio--main"),
        await outcome(
            accounts.user(ids.owner).machine.rename({
                scope: accountId,
                id: identity.machineId,
                requestId: RequestId.create(Date.now()),
                name: "atelier",
            }),
        ),
        await rename("studio"),
    ]).toEqual([
        "executed",
        "CONFLICT: another machine has the same scope and name",
        "executed",
        "BAD_REQUEST: invalid input: name: labels are lowercase letters, digits and single inner hyphens",
        "executed",
        "executed",
    ]);
    expect(
        (await accounts.machine(identity).machine.get({ scope: accountId, id: identity.machineId }))
            .name,
    ).toBe("studio");

    // rotate the machine's key as the machine and refuse the owner registering a key for it
    await identity.rotate(accounts.machine(identity), accountId);
    expect(await itself()).toBe("executed");
    const impostor = new MachineIdentity(identity.machineId, new MemoryKeychain(), Clock.system);
    const { publicKey: foreign, proof } = await impostor.generate();
    expect(
        await outcome(
            accounts.user(ids.owner).key.create({
                scope: accountId,
                requestId: RequestId.create(Date.now()),
                parent: { ...machine.typeReference, id: identity.machineId },
                publicKey: foreign,
                proof,
            }),
        ),
    ).toBe("FORBIDDEN: permission denied: rotate");

    // revoke the machine as the owner
    await accounts.user(ids.owner).machine.revoke({
        scope: accountId,
        id: identity.machineId,
        requestId: RequestId.create(Date.now()),
    });
    expect(await grant(identity)).toBe(NO_STANDING_KEY);
});

test("grant a cloud machine tokens for a minute as itself alone, spending each assertion once, until the machine rotates its key, is disabled or is revoked", async () => {
    // enroll a cloud machine of the residency and store its first key apart
    const accountId = accounts.platform;
    const keys = new MemoryKeychain();
    const identity = new MachineIdentity(Identifier.create("machine"), keys, Clock.system);
    await identity.enroll(accounts.user(ids.operator), {
        scope: accountId,
        requestId: RequestId.create(Date.now()),
        name: "edge",
        kind: "cloud",
        residencyId: "eu",
        providerCode: "cloudflare",
        location: "eu",
    });
    const keychain = new MemoryKeychain();
    keychain.secrets.set(identity.machineId, found(keys.secrets, identity.machineId));
    const first = new MachineIdentity(identity.machineId, keychain, Clock.system);
    const [enrolled] = await accounts.database
        .select()
        .from(key.table)
        .where(eq(key.table.parentId, identity.machineId));
    if (enrolled === undefined) {
        throw new TypeError("the enrolled machine has no key");
    }

    // act as the machine alone
    const fetch = (request: Request) => accounts.fetch(request);
    const accessToken = await identity.token(AUDIENCE, { url: ISSUER }, fetch);
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: ISSUER,
        audience: AUDIENCE,
        keys: await Identity.keys(Snapshot.live(accounts.database), Scope.universe.id),
    });
    const verified = await verifier.authenticate(
        new Request("https://edge.test/spaces", {
            headers: { authorization: `Bearer ${accessToken}` },
        }),
        Date.now(),
    );
    const subject = principal.machine.reference(Scope.universe.id, identity.machineId);
    const { credential } = verified;
    expect([
        { kind: credential.kind, id: credential.id },
        verified.caller.subject,
        verified.caller.groups,
        credential.expiresAt - credential.verifiedAt,
    ]).toEqual([
        { kind: "machine-key", id: enrolled.thumbprint },
        subject,
        [],
        TOKEN_LIFETIME_MILLISECONDS,
    ]);

    // refuse a replayed assertion and one for another endpoint
    const assertion = await identity.assert(ENDPOINT);
    const other = await identity.assert("https://elsewhere.test/oauth2/token");
    expect([await ask(assertion), await ask(assertion), await ask(other)]).toEqual([
        "executed",
        "invalid_grant: assertion was used before",
        "invalid_grant: invalid assertion",
    ]);

    // rotate the key and refuse revoking the first again
    await identity.rotate(accounts.machine(identity), accountId);
    expect(
        await outcome(
            accounts.machine(identity).key.revoke({
                scope: accountId,
                id: enrolled.id,
                requestId: RequestId.create(Date.now()),
            }),
        ),
    ).toBe("CONFLICT: key is revoked");

    // refuse the first key's assertions and accept the new key's
    expect([await grant(first), await grant(identity)]).toEqual([NO_STANDING_KEY, "executed"]);

    // refuse every grant while the machine is disabled and grant again once enabled
    const status = (method: "disable" | "enable") =>
        accounts.user(ids.operator).machine[method]({
            scope: accountId,
            id: identity.machineId,
            requestId: RequestId.create(Date.now()),
        });
    await status("disable");
    const disabled = await grant(identity);
    await status("enable");
    expect([disabled, await grant(identity)]).toEqual([NO_STANDING_KEY, "executed"]);

    // refuse every grant once the operator revokes the machine
    await accounts.user(ids.operator).machine.revoke({
        scope: accountId,
        id: identity.machineId,
        requestId: RequestId.create(Date.now()),
    });
    expect(await grant(identity)).toBe(NO_STANDING_KEY);
});

test("issue a machine's token at the store clock's time, verifiable after the clock advances to it", async () => {
    // run the account package ten minutes ahead of real time
    const clock = new VirtualClock(Date.now()).resume();
    clock.advance(10 * 60_000);
    await using fixture = await AccountFixture.open({ signIn: { clock } });

    // enroll a cloud machine on the same clock and take its token
    const identity = new MachineIdentity(Identifier.create("machine"), new MemoryKeychain(), clock);
    await identity.enroll(fixture.user(ids.operator), {
        scope: fixture.platform,
        requestId: RequestId.create(Date.now()),
        name: "edge",
        kind: "cloud",
        residencyId: "eu",
        providerCode: "cloudflare",
        location: "eu",
    });
    const fetch = (request: Request) => fixture.fetch(request);
    const accessToken = await identity.token(AUDIENCE, { url: ISSUER }, fetch);

    // verify it at the clock's time, with exactly the provider's claims
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: ISSUER,
        audience: AUDIENCE,
        keys: await Identity.keys(Snapshot.live(fixture.database), Scope.universe.id),
    });
    const verified = await verifier.authenticate(
        new Request("https://edge.test/spaces", {
            headers: { authorization: `Bearer ${accessToken}` },
        }),
        clock.now(),
    );
    const verifiedAt = Math.floor(clock.now() / 1000);
    const subject = principal.machine.reference(Scope.universe.id, identity.machineId);
    const { jti, caller, iat = 0, ...claims } = decodeJwt(accessToken);
    expect([verified.caller.subject, verifiedAt - iat <= 1, claims]).toEqual([
        subject,
        true,
        {
            sub: identity.machineId,
            aud: PackageId.resource(AUDIENCE),
            client_id: identity.machineId,
            azp: identity.machineId,
            scope: "",
            iss: ISSUER,
            exp: iat + TOKEN_LIFETIME_MILLISECONDS / 1000,
        },
    ]);
    expect([typeof jti, typeof caller]).toEqual(["string", "object"]);
});

test("reuse a machine's token for several calls to one package, exchanging it once", async () => {
    // count the machine's exchanges at the token endpoint
    const identity = await accounts.enroll(ids.account);
    const exchanged: string[] = [];
    const fetch = (request: Request) => {
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

    expect({
        distinct: new Set([...tokens, later]).size,
        isOther: other !== later,
        exchanged: exchanged.length,
    }).toEqual({
        distinct: 1,
        isOther: true,
        exchanged: 2,
    });
});

test("grant a machine tokens for the spaces it serves, receives or works in, and refuse others", async () => {
    // enroll an account's machine and a cloud machine and place a space on each
    const laptop = await accounts.enroll(ids.account);
    const cloud = await accounts.enroll(accounts.platform, { residencyId: "eu" });
    const own = Identifier.create("space");
    const served = Identifier.create("space");
    const elsewhere = Identifier.create("space");
    const owned = { id: own, scope: ids.account, machine: laptop.machineId, epoch: 1 };
    await TestDirectory.place(accounts.database, owned);
    const placed = { id: served, scope: ids.account, machine: cloud.machineId, epoch: 1 };
    await TestDirectory.place(accounts.database, placed);

    // grant tokens in the spaces each machine serves
    const grantIn = (signer: MachineIdentity, spaceId: Identifier<"space">) =>
        grant(signer, spaceId);
    const refused = [
        await grantIn(laptop, served),
        await grantIn(cloud, elsewhere),
        await grantIn(cloud, own),
    ];

    // grant a token in a space giving the machine work, until the space withdraws it
    await TestDirectory.assign(accounts.database, served, laptop.machineId, true);
    const assigned = await grantIn(laptop, served);
    await TestDirectory.assign(accounts.database, served, laptop.machineId, false);
    const withdrawn = await grantIn(laptop, served);

    // grant a token in a space moving to the machine
    await TestDirectory.place(accounts.database, owned, {
        target: cloud.machineId,
        transferId: Identifier.create("transfer"),
    });
    expect([
        await grantIn(laptop, own),
        await grantIn(cloud, served),
        refused,
        await grantIn(cloud, own),
        [assigned, withdrawn],
    ]).toEqual([
        "executed",
        "executed",
        [
            `invalid_grant: ${served} is served and received elsewhere`,
            `invalid_grant: ${elsewhere} is served and received elsewhere`,
            `invalid_grant: ${own} is served and received elsewhere`,
        ],
        "executed",
        ["executed", `invalid_grant: ${served} is served and received elsewhere`],
    ]);
});

test("disable, drain and enable a machine, refusing its token grants while disabled", async () => {
    const identity = await accounts.enroll(ids.account);
    const change = (userId: string, name: "disable" | "drain" | "enable") =>
        outcome(
            accounts.user(userId).machine[name]({
                scope: ids.account,
                id: identity.machineId,
                requestId: RequestId.create(Date.now()),
            }),
        );
    const verify = () => grant(identity);

    // refuse the machine's grants while disabled and grant them while draining and enabled
    const outcomes = [];
    for (const name of ["disable", "drain", "enable"] as const) {
        outcomes.push(await change(ids.owner, name), await verify());
    }
    expect(outcomes).toEqual([
        "executed",
        NO_STANDING_KEY,
        "executed",
        "executed",
        "executed",
        "executed",
    ]);

    // refuse a stranger changing the status and any change once the machine is revoked
    const stranger = await change(ids.stranger, "disable");
    await accounts.user(ids.owner).machine.revoke({
        scope: ids.account,
        id: identity.machineId,
        requestId: RequestId.create(Date.now()),
    });
    expect([stranger, await change(ids.owner, "enable")]).toEqual([
        `NOT_FOUND: no scope ${ids.account}`,
        "CONFLICT: machine is revoked",
    ]);
});

test("store one key when two processes generate a machine's first key at once, both adopting the one stored first", async () => {
    // generate the first key from two identities sharing one keychain
    const keys = new MemoryKeychain();
    const machineId = Identifier.create("machine");
    const generated = await Promise.all([
        new MachineIdentity(machineId, keys, Clock.system).generate(),
        new MachineIdentity(machineId, keys, Clock.system).generate(),
    ]);

    // answer the stored key's public half to both
    const { kty, crv, x, y } = StoredKey.parse(JSON.parse(found(keys.secrets, machineId)));
    expect(generated.map((entry) => entry.publicKey)).toEqual([
        { kty, crv, x, y },
        { kty, crv, x, y },
    ]);
});

test("rotate once for two identities of one machine rotating at once, ending on the key the universe has as current", async () => {
    // enroll a machine and rotate its key from two of its identities
    const accountId = ids.account;
    const keys = new MemoryKeychain();
    const identity = new MachineIdentity(Identifier.create("machine"), keys, Clock.system);
    await identity.enroll(accounts.user(ids.owner), {
        scope: accountId,
        requestId: RequestId.create(Date.now()),
        name: "racer",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
    });
    const rotations = [
        new MachineIdentity(identity.machineId, keys, Clock.system),
        new MachineIdentity(identity.machineId, keys, Clock.system),
    ];
    await Promise.all(
        rotations.map((rotation) => rotation.rotate(accounts.machine(rotation), accountId)),
    );

    // register one new key and end on the key the universe has not revoked
    const registered = await accounts.database
        .select()
        .from(key.table)
        .where(eq(key.table.parentId, identity.machineId));
    const { kty, crv, x, y } = StoredKey.parse(JSON.parse(found(keys.secrets, identity.machineId)));
    expect({
        registered: registered.length,
        active: registered.filter((key) => key.revokedAt === null).map((key) => key.publicKey),
    }).toEqual({ registered: 2, active: [{ kty, crv, x, y }] });
});
