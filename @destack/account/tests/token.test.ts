import { accessGrant, type GrantPermission, principal } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { Identifier, present, schema } from "@destack/schema";
import { Scope, Subject, ObjectReference } from "@destack/sync";
import { createClient } from "@destack/runtime/client";
import { RequestId } from "@destack/runtime/request";
import { expect, refusal, test } from "@destack/test";
import { account } from "../src/object/account.ts";
import { Identity } from "../src/object/identity.ts";
import { Credential } from "../src/credential/credential.ts";
import { ACCESS_TOKEN_TYPE, PERSONAL_ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { ACCOUNTS_URL, AccountFixture, ISSUER, TestDirectory, outcome } from "../src/test/index.ts";
import { claims } from "./fixture.ts";

/** A package another package declares with permissions for grants. */
const VAULT = Identifier.create("package");

/** One day in milliseconds. */
const DAY = 24 * 60 * 60 * 1000;

/** Issue a token client with its holder's grant, act through it within the grant, revoke both. */
test("issue a token client with its grant, act through it, and revoke both", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, spaceId } = await fixture.createSpace(owner);

    // ask for a token reading the account and one space's secrets
    const credential = await Credential.create("personal-access-token");
    const permissions: GrantPermission[] = [
        { ...account.permission("read"), scope: accountId },
        { packageId: VAULT, type: "version", name: "read", scope: spaceId },
    ];
    const expiresAt = Date.now() + DAY;
    const issuing = {
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "agent",
        digest: credential.digest,
        permissions,
        expiresAt,
    };

    // challenge a single factor
    expect(await outcome(owner.client.client.issue(issuing))).toBe(
        "INSUFFICIENT_AUTHENTICATION: authentication assurance is too low: authenticate again",
    );
    await fixture.elevate(owner.id);
    const issued = await owner.client.client.issue(issuing);
    expect(issued).toEqual({
        id: issued.id,
        createdAt: issued.createdAt,
        ...ObjectReference.columns("createdBy", owner.subject),
        ...ObjectReference.columns("updatedBy", owner.subject),
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

    // grant the token the permissions until its expiry, from the owner in their own scope
    const token = principal.client.reference(Scope.universe.id, issued.id);
    const grants = async () => (await owner.client.grant.list({ userId: owner.id })).items;
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

    // expect the prefix before 32 random bytes in canonical base64url
    const encoded = credential.secret.slice("dst_pat_".length);
    const bytes = Uint8Array.fromBase64(encoded, { alphabet: "base64url" });
    expect({
        prefix: credential.secret.slice(0, "dst_pat_".length),
        length: bytes.length,
        encoded: bytes.toBase64({ alphabet: "base64url", omitPadding: true }),
    }).toEqual({ prefix: "dst_pat_", length: 32, encoded });

    // exchange the token for the owner's tokens through it, carrying its grant by reference
    const exchange = (space?: Identifier<"space">) =>
        fixture.exchangeToken(credential.secret, PERSONAL_ACCESS_TOKEN_TYPE, {
            audience: space === undefined ? accountPackage.id : VAULT,
            ...(space === undefined ? {} : { space }),
        });
    const universal = claims((await exchange()).accessToken);
    const exchanged = claims((await exchange(spaceId)).accessToken);
    expect([
        [universal.sub, universal.act, universal["grants"]],
        [exchanged.sub, exchanged.act, exchanged["grants"]],
    ]).toEqual([
        [owner.id, [token], [{ id: grantId, scope: owner.id }]],
        [owner.id, [token], [{ id: grantId, scope: owner.id }]],
    ]);
    const bearer = fixture.bearer((await exchange()).accessToken);

    // find no owner's scope at all, which the grant leaves out
    expect(await refusal(bearer.client.list({ userId: owner.id }))).toEqual([
        "NOT_FOUND",
        `no scope ${owner.id}`,
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
            { expiresAt: Date.now() - 1 },
            { expiresAt: Date.now() + 400 * DAY },
        ].map((change) =>
            outcome(
                owner.client.client.issue({
                    ...issuing,
                    requestId: RequestId.create(Date.now()),
                    name: "refused",
                    ...change,
                }),
            ),
        ),
    );
    expect(refusals).toEqual([
        "BAD_REQUEST: unknown permission account.fly",
        "BAD_REQUEST: token cannot act in elsewhere",
        "BAD_REQUEST: token expiry must lie within 366 days from now",
        "BAD_REQUEST: token expiry must lie within 366 days from now",
    ]);

    // hide the token from another user, who finds the owner's public profile alone
    const other = await fixture.person("other@example.com");
    expect((await other.client.client.list({ userId: owner.id })).items).toEqual([]);
    expect(
        await outcome(
            other.client.client.issue({
                ...issuing,
                userId: owner.id,
                requestId: RequestId.create(Date.now()),
                name: "stolen",
                digest: (await Credential.create("personal-access-token")).digest,
                permissions: [],
            }),
        ),
    ).toBe("NOT_FOUND: client not found");

    // revoke the token with its grant, refusing both at once
    const revoking = { userId: owner.id, id: issued.id };
    const revoked = await owner.client.client.revoke({
        ...revoking,
        requestId: RequestId.create(Date.now()),
    });
    expect({
        revokedAt: schema.looseObject({ revokedAt: schema.number() }).parse(revoked).revokedAt > 0,
        grants: await grants(),
        exchanged: await outcome(exchange()),
        revoked: await outcome(
            owner.client.client.revoke({
                ...revoking,
                requestId: RequestId.create(Date.now()),
            }),
        ),
    }).toEqual({
        revokedAt: true,
        grants: [],
        exchanged: "BAD_REQUEST: invalid_grant: invalid personal access token",
        revoked: "CONFLICT: client is revoked",
    });
});

/** Stop a token authenticating once its grant lapses. */
test("refuse a personal access token whose grant lapsed", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);

    // issue a token reading the account
    await fixture.elevate(owner.id);
    const credential = await Credential.create("personal-access-token");
    await owner.client.client.issue({
        userId: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "agent",
        digest: credential.digest,
        permissions: [{ ...account.permission("read"), scope: accountId }],
        expiresAt: Date.now() + DAY,
    });
    const exchange = async () =>
        claims(
            (
                await fixture.exchangeToken(credential.secret, PERSONAL_ACCESS_TOKEN_TYPE, {
                    audience: accountPackage.id,
                })
            ).accessToken,
        ).sub;
    expect(await exchange()).toBe(owner.id);

    // lapse its grant just after its creation
    const [grant] = (await owner.client.grant.list({ userId: owner.id })).items;
    const { id, createdAt } = present(grant, "the token's grant");
    await fixture.storage.database
        .update(accessGrant)
        .set({ expiresAt: createdAt + 1 })
        .where(eq(accessGrant.id, id));
    expect(await outcome(exchange())).toBe(
        "BAD_REQUEST: invalid_grant: invalid personal access token",
    );
});

/** Exchange an installation's token for its account's spaces, refusing other accounts'. */
test("exchange an installation's token for tokens of its account's spaces with its account grants", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, spaceId, machine } = await fixture.createSpace(owner);
    const elsewhere = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);

    // place a second space in the account, and make an installation of the first its editor
    const sibling = Identifier.create("space");
    await TestDirectory.place(fixture.storage.database, {
        id: sibling,
        scope: accountId,
        machine: Identifier.create("machine"),
        epoch: 1,
    });
    const automation = principal.installation.reference(spaceId, Identifier.create("installation"));
    await owner.client.account.grant({
        scope: owner.id,
        id: accountId,
        requestId: RequestId.create(Date.now()),
        relation: "editor",
        subject: automation,
    });

    // exchange its token for a token of the second space
    const exchange = async (space: Identifier<"space">, subject?: string) =>
        fixture.exchangeToken(await fixture.installationToken(automation), ACCESS_TOKEN_TYPE, {
            audience: VAULT,
            space,
            ...(subject === undefined ? {} : { subject }),
        });
    const { caller } = claims((await exchange(sibling)).accessToken);
    const onAccount = account.reference(owner.id, accountId);
    expect([caller.subject, caller.groups]).toEqual([
        automation,
        [
            { ...onAccount, relation: "editor" },
            { ...onAccount, relation: "member" },
        ],
    ]);

    // refuse acting as a user or in another account's spaces
    expect([
        await outcome(exchange(spaceId, Subject.key(owner.subject))),
        await outcome(exchange(elsewhere.spaceId)),
    ]).toEqual([
        `BAD_REQUEST: invalid_grant: ${owner.id} has no grant to ${automation.id}`,
        "BAD_REQUEST: invalid_grant: the installation belongs to another account",
    ]);
});

test("follow a space's chain at the universe as the space, through the tokens its issuer grants its assertions", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, spaceId, machine } = await fixture.createSpace(owner);
    await fixture.startIdentity(spaceId, machine);

    // call the account package as the space with tokens granted for its assertions
    const space = principal.space.reference(Scope.universe.id, spaceId);
    const accounts = createClient(accountPackage, {
        url: ACCOUNTS_URL,
        fetch: fixture.keystore
            .source(fixture.database, space, {
                issuer: ISSUER,
                send: (request) => fixture.fetch(request),
            })
            .fetchGranted(accountPackage.id),
    });

    // read the account's scope in the first page of the space's chain replica
    const [chain] = await fixture.objects.authorizer.chain(
        fixture.database,
        spaceId,
        { kind: "home", parent: accountId },
        fixture.objects.source.reached(),
    );
    const stream = await accounts.store.stream(present(chain, "the space's chain"), {
        signal: AbortSignal.timeout(5000),
    });
    const [page] = await Array.fromAsync(take(stream, 1));
    expect(
        present(page, "the first page")
            .changes.filter((change) => change.table === "destack__sync__scope")
            .map((change) => change.row["scope"]),
    ).toEqual([accountId]);
});

/** Take the first items of an iterable. */
async function* take<Item>(items: AsyncIterable<Item>, count: number): AsyncGenerator<Item> {
    let taken = 0;
    for await (const item of items) {
        yield item;
        taken += 1;
        if (taken === count) {
            return;
        }
    }
}

test("authenticate a person's universe token in the space it is bound to, refusing it in another and calling anonymously without one", async () => {
    // sign a person's token toward the account package in their home
    await using fixture = await AccountFixture.open();
    const person = await fixture.person("ada@example.test");
    const { spaceId } = await fixture.personalHome(person);
    const token = await fixture.token(person.id, accountPackage.id, spaceId);

    // authenticate calls to the home and to another space over the universe's directory
    const authenticate = (scope: string, headers: Record<string, string>) =>
        Identity.authenticate(
            Snapshot.live(fixture.database),
            new Request("https://machine.test/", { headers }),
            {
                audience: accountPackage.id,
                universe: ISSUER,
                scope,
                now: Date.now(),
            },
        );
    const bearer = { authorization: `Bearer ${token}` };
    expect({
        anonymous: await authenticate(spaceId, {}),
        home: (await authenticate(spaceId, bearer))?.caller.subject,
        other: await refusal(authenticate(Identifier.create("space"), bearer)),
    }).toEqual({
        anonymous: undefined,
        home: person.subject,
        other: ["UNAUTHORIZED", "invalid access token claims"],
    });
});
