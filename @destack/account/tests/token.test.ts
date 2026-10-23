import objectPackage from "@destack/object/package";
import { type GrantPermission, principal } from "@destack/access";
import { Snapshot } from "@destack/db";
import { Identifier, present } from "@destack/schema";
import { PackageId } from "@destack/package";
import { type AccessToken, TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { Scope, Subject, ObjectReference, RequestId } from "@destack/sync";
import { ObjectSink } from "@destack/object/server";
import { expect, refusal, single, test } from "@destack/test";
import { account } from "../src/object/account.ts";
import { Identity } from "../src/object/identity.ts";
import { Credential } from "../src/credential/credential.ts";
import { PERSONAL_ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import {
    TestAccountStore,
    TestJwt,
    ISSUER,
    OWNER_EMAIL,
    STRANGER_EMAIL,
} from "../src/test/index.ts";
import { ANY_NUMBER, ANY_STRING, firstPage } from "./fixture.ts";

/** A package another package declares with permissions for grants. */
const VAULT = Identifier.create("package");

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Read the universe's position an access token is stamped with. */
function stampOf(claims: AccessToken): number {
    return single(present(claims.caller.credential.watermarks, "the watermarks")).sequence;
}

test("issue a token client with its grant, act through it, and revoke both", async () => {
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    let owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId } = await fixture.createSpace(owner);

    // ask for a token reading the account and one space's secrets
    const credential = await Credential.create("personal-access-token");
    const permissions: GrantPermission[] = [
        { ...account.permission("read"), scope: accountId },
        { packageId: VAULT, type: "version", name: "read", scope: spaceId },
    ];
    const expiresAt = fixture.store.clock.now() + DAY;
    const issuing = {
        userId: owner.id,
        requestId: RequestId.create(fixture.store.clock.now()),
        name: "agent",
        digest: credential.digest,
        permissions,
        expiresAt,
    };

    // challenge a single factor
    expect(await refusal(owner.client(accountPackage).client.issue(issuing))).toEqual([
        "UNAUTHORIZED",
        "INSUFFICIENT_AUTHENTICATION",
        "authentication assurance is too low: authenticate again",
    ]);
    owner = await fixture.elevate(owner);
    const issued = await owner.client(accountPackage).client.issue(issuing);
    expect(issued).toEqual({
        id: issued.id,
        createdAt: issued.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        updatedAt: issued.createdAt,
        revision: 1,
        tags: {},
        scope: owner.id,
        name: "agent",
        kind: "token",
        machineId: null,
        platform: null,
        platformVersion: null,
        architecture: null,
        model: null,
        version: null,
        lastActiveAt: issued.createdAt,
        revokedAt: null,
    });

    // grant the token the permissions until its expiry, from the owner in their scope
    const token = principal.client.reference(Scope.universe.id, issued.id);
    const grants = async () =>
        (await owner.client(objectPackage).grant.list({ scope: owner.id })).items;
    const [grant] = await grants();
    expect(await grants()).toEqual([
        {
            id: present(grant, "the token's grant").id,
            createdAt: issued.createdAt,
            revision: 1,
            scope: owner.id,
            ...ObjectReference.columns("delegator", owner.subject),
            ...ObjectReference.columns("delegate", token),
            permissions,
            expiresAt,
        },
    ]);
    const grantId = present(grant, "the token's grant").id;

    // exchange the token for the owner's tokens through it, sending its grant by reference
    const exchange = (space?: Identifier<"space">) =>
        fixture.issuer.exchange(
            { token: credential.secret, type: PERSONAL_ACCESS_TOKEN_TYPE },
            space === undefined ? accountPackage.id : VAULT,
            space === undefined ? {} : { spaceId: space },
        );
    const issuedAt = Math.floor(fixture.clock.now() / 1000);
    const acting = (
        audience: string,
        groups: readonly object[],
        position: { readonly log: string; readonly epoch: string },
    ) => ({
        iss: ISSUER,
        sub: owner.id,
        aud: audience,
        client_id: issued.id,
        azp: issued.id,
        scope: "",
        jti: ANY_STRING,
        iat: issuedAt,
        exp: issuedAt + TOKEN_LIFETIME_MILLISECONDS / 1000,
        act: [token],
        grants: [{ id: grantId, scope: owner.id }],
        caller: {
            subject: owner.subject,
            groups,
            contacts: [],
            credential: {
                kind: "personal-access-token",
                id: issued.id,
                watermarks: [
                    {
                        scope: Scope.universe.id,
                        log: position.log,
                        epoch: position.epoch,
                        sequence: ANY_NUMBER,
                    },
                ],
            },
        },
    });

    // stamp each token with the universe's position at its exchange, and the account's groups in its space
    const { log } = fixture.database;
    const universalAt = await log.position();
    const universal = TestJwt.claims((await exchange()).accessToken);
    const spacedAt = await log.position();
    const spaced = TestJwt.claims((await exchange(spaceId)).accessToken);
    const exchangedAt = await log.position();
    const onAccount = account.reference(owner.id, accountId);
    expect([universal, spaced]).toEqual([
        acting(PackageId.resource(accountPackage.id), [], universalAt),
        {
            ...acting(
                PackageId.resource(VAULT),
                [
                    { ...onAccount, relation: "member" },
                    { ...onAccount, relation: "owner" },
                ],
                spacedAt,
            ),
            space: spaceId,
        },
    ]);
    const [universalStamp, spacedStamp] = [stampOf(universal), stampOf(spaced)];
    expect(universalStamp).toBeGreaterThanOrEqual(universalAt.sequence);
    expect(spacedStamp).toBeGreaterThan(universalStamp);
    expect(spacedStamp).toBeGreaterThanOrEqual(spacedAt.sequence);
    expect(spacedStamp).toBeLessThanOrEqual(exchangedAt.sequence);
    const bearer = fixture.issuer.accounts((await exchange()).accessToken);

    // find no owner's scope at all, which the grant leaves out
    expect(await refusal(bearer.client.list({ userId: owner.id }))).toEqual([
        "NOT_FOUND",
        "SCOPE_NOT_FOUND",
        `scope ${owner.id} not found`,
    ]);

    // refuse unknown account permissions, unknown scopes and unbounded expiries
    const refusals = await Promise.all(
        [
            { permissions: [{ ...account.permission("read"), name: "fly", scope: accountId }] },
            {
                permissions: [
                    { packageId: VAULT, type: "version", name: "read", scope: "elsewhere" },
                ],
            },
            { expiresAt: fixture.store.clock.now() - 1 },
            { expiresAt: fixture.store.clock.now() + 400 * DAY },
        ].map((change) =>
            refusal(
                owner.client(accountPackage).client.issue({
                    ...issuing,
                    requestId: RequestId.create(fixture.store.clock.now()),
                    name: "refused",
                    ...change,
                }),
            ),
        ),
    );
    expect(refusals).toEqual([
        ["BAD_REQUEST", "UNKNOWN_PERMISSION", "unknown permission account.fly"],
        ["BAD_REQUEST", "UNUSABLE_SCOPE", "token cannot act in elsewhere"],
        ["BAD_REQUEST", "INVALID_TOKEN_EXPIRY", "token expiry must be within 366 days from now"],
        ["BAD_REQUEST", "INVALID_TOKEN_EXPIRY", "token expiry must be within 366 days from now"],
    ]);

    // hide the token from another user, who finds the owner's public profile alone
    const other = await fixture.signIn(STRANGER_EMAIL);
    expect((await other.client(accountPackage).client.list({ userId: owner.id })).items).toEqual(
        [],
    );
    expect(
        await refusal(
            other.client(accountPackage).client.issue({
                ...issuing,
                userId: owner.id,
                requestId: RequestId.create(fixture.store.clock.now()),
                name: "stolen",
                digest: (await Credential.create("personal-access-token")).digest,
                permissions: [],
            }),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", "client not found"]);

    // revoke the token with its grant, refusing both at once
    const revoking = { userId: owner.id, id: issued.id };
    const revoked = await owner.client(accountPackage).client.revoke({
        ...revoking,
        requestId: RequestId.create(fixture.store.clock.now()),
    });
    expect({
        revoked,
        grants: await grants(),
        exchanged: await refusal(exchange()),
        again: await refusal(
            owner.client(accountPackage).client.revoke({
                ...revoking,
                requestId: RequestId.create(fixture.store.clock.now()),
            }),
        ),
    }).toEqual({
        revoked: {
            ...issued,
            revokedAt: fixture.clock.now(),
            updatedAt: fixture.clock.now(),
            revision: 2,
        },
        grants: [],
        exchanged: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: invalid personal access token",
        ],
        again: ["CONFLICT", "REVOKED", "client is revoked"],
    });
});

test("refuse a personal access token whose grant lapsed", async () => {
    await using fixture = await TestAccountStore.open();
    const { clock } = fixture;
    let owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);

    // issue a token reading the account
    owner = await fixture.elevate(owner);
    const credential = await Credential.create("personal-access-token");
    await owner.client(accountPackage).client.issue({
        userId: owner.id,
        requestId: RequestId.create(fixture.store.clock.now()),
        name: "agent",
        digest: credential.digest,
        permissions: [{ ...account.permission("read"), scope: accountId }],
        expiresAt: fixture.store.clock.now() + DAY,
    });
    const exchange = async () =>
        TestJwt.claims(
            (
                await fixture.issuer.exchange(
                    { token: credential.secret, type: PERSONAL_ACCESS_TOKEN_TYPE },
                    accountPackage.id,
                    {},
                )
            ).accessToken,
        ).sub;
    expect(await exchange()).toBe(owner.id);

    // let its grant lapse
    clock.pause();
    clock.advance(DAY + 1);
    expect(await refusal(exchange())).toEqual([
        "BAD_REQUEST",
        "INVALID_GRANT",
        "the token endpoint refused the request: invalid_grant: invalid personal access token",
    ]);
});

test("exchange an installation's token for tokens of its account's spaces with its account grants", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId, machine } = await fixture.createSpace(owner);
    const elsewhere = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);

    // place a second space in the account, and make an installation of the first its editor
    const sibling = Identifier.create("space");
    const siblingMachine = await fixture.enroll(accountId, owner);
    await owner.client(accountPackage).space.create({
        accountId,
        id: sibling,
        requestId: RequestId.create(fixture.store.clock.now()),
        name: `space-${sibling.slice(-12)}`,
        machine: siblingMachine.machineId,
    });
    const automation = principal.installation.reference(spaceId, Identifier.create("installation"));
    await owner.client(accountPackage).account.grant({
        scope: owner.id,
        id: accountId,
        requestId: RequestId.create(fixture.store.clock.now()),
        relation: "editor",
        subject: automation,
    });

    // exchange its token for a token of the second space
    const exchange = async (space: Identifier<"space">, subject?: Subject) =>
        fixture.source(automation).exchange(VAULT, {
            spaceId: space,
            ...(subject === undefined ? {} : { subject }),
        });
    const { credential, ...identity } = TestJwt.claims(
        (await exchange(sibling)).accessToken,
    ).caller;
    const onAccount = account.reference(owner.id, accountId);
    expect([identity, credential.kind]).toEqual([
        {
            subject: automation,
            groups: [
                { ...onAccount, relation: "editor" },
                { ...onAccount, relation: "member" },
            ],
            contacts: [],
        },
        "installation-key",
    ]);

    // refuse acting as a user or in another account's spaces
    expect([
        await refusal(exchange(spaceId, owner.subject)),
        await refusal(exchange(elsewhere.spaceId)),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "INVALID_GRANT",
            `the token endpoint refused the request: invalid_grant: ${owner.id} has no grant to ${automation.id}`,
        ],
        [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: the installation belongs to another account",
        ],
    ]);
});

test("follow a space's chain at the universe as the space, through the tokens its issuer grants its assertions", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);

    // follow the account's scope in the space's chain replica as the space, by its granted tokens
    const chain = fixture.store.source.chainPublication.subscription({
        name: ObjectSink.chainName(spaceId),
        scope: accountId,
        target: spaceId,
        parameters: { scopes: [spaceId, accountId], local: [], types: [], via: [], between: [] },
    });
    const controller = new AbortController();
    const followed = fixture
        .as(spaceId)
        .client(accountPackage)
        .store.stream(chain, { signal: controller.signal })
        .then((pages) => firstPage(pages));

    // answer its first page
    expect(await refusal(followed).finally(() => controller.abort())).toBe("done");
});

test("authenticate a person's universe token in the space it is bound to, refusing it in another and calling anonymously without one", async () => {
    // sign a person's token toward the account package in their home
    await using fixture = await TestAccountStore.open();
    const person = await fixture.signIn("person@example.com");
    const { spaceId } = await fixture.personalHome(person);
    const { accessToken: token } = await person.exchange(accountPackage.id, { spaceId });

    // authenticate calls to the home and to another space over the universe's directory
    const authenticate = (scope: string, headers: Record<string, string>) =>
        Identity.authenticate(
            Snapshot.live(fixture.database),
            new Request("https://machine.test/", { headers }),
            {
                audience: accountPackage.id,
                universe: ISSUER,
                scope,
                now: fixture.store.clock.now(),
            },
        );
    const bearer = { authorization: `Bearer ${token}` };
    const home = present(await authenticate(spaceId, bearer), "the home's authentication");
    expect({
        anonymous: await authenticate(spaceId, {}),
        home: home.caller.subject,
        other: await refusal(authenticate(Identifier.create("space"), bearer)),
    }).toEqual({
        anonymous: undefined,
        home: person.subject,
        other: ["UNAUTHORIZED", "INVALID_TOKEN", "invalid access token claims"],
    });
});
