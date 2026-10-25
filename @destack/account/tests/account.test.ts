import { ObjectReference, RequestId } from "@destack/sync";
import { Account, account } from "../src/object/account.ts";
import { expect, refusal, test } from "@destack/test";
import { eq, Snapshot } from "@destack/db";
import accountPackage from "../src/package.ts";
import { ANY_STRING } from "./fixture.ts";
import { placement } from "../src/object/placement.ts";
import { PackageId } from "@destack/package";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import {
    ids,
    ISSUER,
    SIGN_UP,
    TestAccountStore,
    TestJwt,
    type TestPerson,
    OWNER_EMAIL,
    STACKS,
    STRANGER_EMAIL,
} from "../src/test/index.ts";
import { type Identifier, present, schema } from "@destack/schema";
import { space } from "../src/object/space.ts";

test("rename a user as themselves, the user created and changed on the store's clock", async () => {
    // sign the user up and rename them at a clock standing still
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const signedUpAt = fixture.clock.now();
    const owner = await fixture.signIn("newcomer@example.test");
    const client = owner.client(accountPackage);
    const person = await client.user.get({ id: owner.id });
    const renamed = await client.user.update({
        id: owner.id,
        requestId: RequestId.create(fixture.environment),
        revision: person.revision,
        name: "Florian",
    });

    // record the sign-up on the store's clock, and store the new name in a new revision
    expect([person.createdAt, person.updatedAt]).toEqual([signedUpAt, signedUpAt]);
    expect(renamed).toEqual({
        ...person,
        ...ObjectReference.columns("updatedBy", owner.subject),
        name: "Florian",
        revision: person.revision + 1,
        updatedAt: fixture.clock.now(),
    });
});

test("change a user's time zone and locale, and refuse an unknown time zone", async () => {
    // change the time zone to an unknown one, then to a known one with a locale
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const users = owner.client(accountPackage).user;
    const current = await users.get({ id: owner.id });
    const change = { id: owner.id, revision: current.revision };
    const unknown = await refusal(
        users.update({
            ...change,
            requestId: RequestId.create(fixture.environment),
            timeZone: "Mars/Olympus",
        }),
    );
    const changed = await users.update({
        ...change,
        requestId: RequestId.create(fixture.environment),
        timeZone: "Europe/London",
        locale: "en-GB",
    });

    // store the known one alone
    expect({ unknown, changed }).toEqual({
        unknown: ["BAD_REQUEST", "TIME_ZONE_UNKNOWN", "unknown time zone Mars/Olympus"],
        changed: {
            ...current,
            ...ObjectReference.columns("updatedBy", owner.subject),
            timeZone: "Europe/London",
            locale: "en-GB",
            revision: current.revision + 1,
            updatedAt: fixture.clock.now(),
        },
    });
});

test("create a person's shared account beside their personal one, and refuse a second personal one", async () => {
    // create a shared account, and another personal one
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn("person@example.com");
    const client = owner.client(accountPackage);
    const creation = {
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
        handle: "florian",
        name: "Personal",
        residencyId: "eu" as const,
        kind: "shared" as const,
    };
    const shared = await client.account.create(creation);
    const personal = await refusal(
        client.account.create({
            ...creation,
            requestId: RequestId.create(fixture.environment),
            handle: "second",
            kind: "personal",
        }),
    );

    // list both accounts, and store the shared one as created
    const { items } = await client.account.list({ scope: owner.id, limit: 10 });
    expect({
        shared,
        personal,
        listed: items.map((item) => [item.handle, item.kind]),
    }).toEqual({
        shared: {
            id: shared.id,
            createdAt: shared.createdAt,
            ...ObjectReference.columns("createdBy", owner.subject),
            updatedAt: shared.createdAt,
            ...ObjectReference.columns("updatedBy", owner.subject),
            managerInstallationId: null,
            managerPackageId: null,
            managerName: null,
            detachedAt: null,
            ...ObjectReference.columns("deletedBy", undefined),
            revision: 1,
            tags: {},
            handle: "florian",
            name: "Personal",
            residencyId: "eu",
            kind: "shared",
            deletedAt: null,
            scope: owner.id,
        },
        personal: ["CONFLICT", "DUPLICATE", "another account has the same scope"],
        listed: [
            [owner.handle, "personal"],
            ["florian", "shared"],
        ],
    });
});

test("give a shared account's home the account stack, and a person's home the person stack", async () => {
    // sign a person up, then create a shared account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn("person@example.com");
    const shared = await owner.client(accountPackage).account.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
        handle: "shared-home",
        name: "Shared",
        residencyId: "eu",
        kind: "shared",
    });

    // read the stack each account's home was created with
    const stackOf = async (accountId: Identifier<"account">) => {
        const home = await Snapshot.live(fixture.database)
            .read(space.table)
            .findFirst({ where: { id: Account.home(accountId) } });

        return present(home, `the home of ${accountId}`).stack;
    };
    expect([await stackOf(shared.id), await stackOf(owner.account)]).toEqual([
        STACKS.account,
        STACKS.person,
    ]);
});

test("show a person's public profile to another signed-in user, and hide their accounts", async () => {
    // create a shared account of the owner
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const shared = await fixture.createAccount(owner);
    const outsider = await fixture.signIn(STRANGER_EMAIL);
    const outsiders = outsider.client(accountPackage);

    // read, rename, create in and list the owner's accounts as the outsider
    const hidden = await Promise.all(
        [
            outsiders.user.get({ id: owner.id }),
            outsiders.account.get({ scope: owner.id, id: shared.id }),
            outsiders.account.update({
                scope: owner.id,
                requestId: RequestId.create(fixture.environment),
                id: shared.id,
                revision: shared.revision,
                name: "Home",
            }),
            outsiders.account.create({
                scope: owner.id,
                requestId: RequestId.create(fixture.environment),
                handle: "intruder",
                name: "Intruder",
                residencyId: "eu",
                kind: "shared",
            }),
        ].map((attempt) => refusal(attempt)),
    );
    const listed = await outsiders.account.list({ scope: owner.id, limit: 10 });
    expect({ hidden, listed: listed.items }).toEqual({
        hidden: [
            "done",
            ["NOT_FOUND", "NOT_FOUND", `account ${shared.id} not found`],
            ["NOT_FOUND", "NOT_FOUND", `account ${shared.id} not found`],
            ["NOT_FOUND", "NOT_FOUND", "account not found"],
        ],
        listed: [],
    });
});

test("refuse exchanging a person's token once they revoke the client it was issued to", async () => {
    // revoke the client the owner signed in on
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    await owner.client(accountPackage).client.revoke({
        userId: owner.id,
        id: owner.clientId,
        requestId: RequestId.create(fixture.environment),
    });

    // refuse the next exchange
    expect(
        await refusal(fixture.issuer.exchange(await owner.token(), accountPackage.id, {})),
    ).toEqual([
        "BAD_REQUEST",
        "INVALID_GRANT",
        "the token endpoint refused the request: invalid_grant: client is revoked",
    ]);
});

test("exchange tokens acting as a user through lent authority or account administration", async () => {
    await using fixture = await TestAccountStore.open();
    let administrator = await fixture.signIn("administrator@example.com");
    let member = await fixture.signIn("member@example.com");
    let assistant = await fixture.signIn("assistant@example.com");

    // create an account with a space and make the member belong to it
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    await administrator.client(accountPackage).account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(fixture.store.environment),
        relation: "editor",
        subject: member.subject,
    });

    // read who a token represents and who acts in it
    const exchange = async (acting: TestPerson, partition = spaceId) => {
        const token = await fixture.issuer.exchange(await acting.token(), accountPackage.id, {
            partition,
            subject: member.subject,
        });
        const claimed = TestJwt.claims(token.accessToken);

        return [claimed.caller.subject, claimed.act];
    };
    const acting = (who: TestPerson) => [member.subject, [who.subject]];

    // let the administrator act as the member only once elevated
    const refused = [
        "BAD_REQUEST",
        "INVALID_GRANT",
        "the token endpoint refused the request: invalid_grant: acting as another user is not allowed",
    ];
    expect(await refusal(exchange(administrator))).toEqual(refused);
    administrator = await fixture.elevate(administrator);
    expect(await exchange(administrator)).toEqual(acting(administrator));

    // refuse the administrator acting as the member in another account's space
    const elsewhere = await fixture.createSpace(administrator);
    expect(await refusal(exchange(administrator, elsewhere.spaceId))).toEqual(refused);

    // let the assistant act as the member only with lent authority and elevation
    expect(await refusal(exchange(assistant))).toEqual(refused);
    const lending = {
        id: member.id,
        requestId: RequestId.create(fixture.store.environment),
        relation: "delegate",
        subject: assistant.subject,
        expiresAt: fixture.store.clock.now() + 60_000,
    };
    expect(await refusal(member.client(accountPackage).user.grant(lending))).toEqual([
        "UNAUTHORIZED",
        "AUTHENTICATION_INSUFFICIENT",
        "authentication assurance is too low: authenticate again",
    ]);
    member = await fixture.elevate(member);
    const lent = await member.client(accountPackage).user.grant(lending);
    assistant = await fixture.elevate(assistant);
    expect(await exchange(assistant)).toEqual(acting(assistant));
    await member.client(accountPackage).user.revoke({
        id: member.id,
        requestId: RequestId.create(fixture.store.environment),
        relationshipId: schema.identifier("relationship").parse(lent.id),
    });
    expect(await refusal(exchange(assistant))).toEqual(refused);
});

test("exchange a token acting as a member in the account's space, with the member's memberships and the acting administrator", async () => {
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    let administrator = await fixture.signIn("administrator@example.com");
    const member = await fixture.signIn("member@example.com");

    // create an account's space and make the member belong to it
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    await administrator.client(accountPackage).account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(fixture.store.environment),
        relation: "editor",
        subject: member.subject,
    });
    administrator = await fixture.elevate(administrator);

    // exchange the administrator's session for a token acting as the member in the space
    const acting = await fixture.issuer.exchange(await administrator.token(), accountPackage.id, {
        partition: spaceId,
        subject: member.subject,
    });
    const signedIn = TestJwt.claims((await administrator.token()).token).caller.credential;
    const assurance = present(signedIn.assurance, "the administrator's assurance");

    // read the member, their memberships, the administrator's assurance and the administrator acting
    const issuedAt = Math.floor(fixture.clock.now() / 1000);
    expect(TestJwt.claims(acting.accessToken)).toEqual({
        iss: ISSUER,
        sub: member.id,
        aud: PackageId.resource(accountPackage.id),
        client_id: administrator.clientId,
        azp: administrator.clientId,
        scope: "",
        jti: ANY_STRING,
        iat: issuedAt,
        exp: issuedAt + TOKEN_LIFETIME_MILLISECONDS / 1000,
        partition: spaceId,
        act: [administrator.subject],
        caller: {
            subject: member.subject,
            groups: [
                { ...account.reference(administrator.id, accountId), relation: "editor" },
                { ...account.reference(administrator.id, accountId), relation: "member" },
            ],
            contacts: [],
            credential: {
                kind: "session",
                id: signedIn.id,
                assurance: { level: 2, authenticatedAt: assurance.authenticatedAt },
            },
        },
    });
});

test("resolve the paying account: an organization's payer for it and its accounts, and a person's account itself", async () => {
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");

    // create an organization paying through its account, with a second account in it
    const acme = await founder.client(accountPackage).organization.create({
        requestId: RequestId.create(fixture.store.environment),
        name: "Acme",
        handle: "acme-pays",
        residencyId: "us",
    });
    const second = await founder.client(accountPackage).account.create({
        scope: acme.id,
        requestId: RequestId.create(fixture.store.environment),
        handle: "acme-second",
        name: "Acme",
        kind: "shared",
        residencyId: "us",
    });

    // charge the organization and both its accounts to its payer, and the person's account to itself
    const snapshot = Snapshot.live(fixture.database);
    expect([
        await Account.payer(snapshot, acme.id),
        await Account.payer(snapshot, second.id),
        await Account.payer(snapshot, acme.payer),
        await Account.payer(snapshot, founder.account),
    ]).toEqual([acme.payer, acme.payer, acme.payer, founder.account]);
});

test("create accounts in an organization, administered by its owner members", async () => {
    await using fixture = await TestAccountStore.open();
    let founder = await fixture.signIn("founder@example.com");
    const partner = await fixture.signIn("partner@example.com");

    // create an organization with its account, and another account in it
    const acme = await founder.client(accountPackage).organization.create({
        requestId: RequestId.create(fixture.store.environment),
        name: "Acme",
        handle: "acme-inc",
        residencyId: "us",
    });
    const created = await founder.client(accountPackage).account.create({
        scope: acme.id,
        requestId: RequestId.create(fixture.store.environment),
        handle: "initech",
        name: "Acme",
        kind: "shared",
        residencyId: "us",
    });
    expect(created).toEqual({
        id: created.id,
        createdAt: created.createdAt,
        ...ObjectReference.columns("createdBy", founder.subject),
        updatedAt: created.createdAt,
        ...ObjectReference.columns("updatedBy", founder.subject),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
        ...ObjectReference.columns("deletedBy", undefined),
        revision: 1,
        tags: {},
        handle: "initech",
        name: "Acme",
        residencyId: "us",
        kind: "shared",
        deletedAt: null,
        scope: acme.id,
    });

    // hide the account from the partner until the partner is an owner
    const rename = (person: TestPerson) =>
        person.client(accountPackage).account.update({
            scope: acme.id,
            id: created.id,
            requestId: RequestId.create(fixture.store.environment),
            revision: created.revision,
            name: "Acme Inc",
        });
    expect(await refusal(rename(partner))).toEqual([
        "NOT_FOUND",
        "SCOPE_NOT_FOUND",
        `scope ${acme.id} not found`,
    ]);
    founder = await fixture.elevate(founder);
    await founder.client(accountPackage).organization.update({
        id: acme.id,
        requestId: RequestId.create(fixture.store.environment),
        payer: created.id,
    });
    await fixture.join(founder, partner, acme.id, "owner");
    const renamed = await rename(partner);
    expect(renamed).toEqual({
        ...created,
        name: "Acme Inc",
        revision: 2,
        updatedAt: renamed.updatedAt,
        ...ObjectReference.columns("updatedBy", partner.subject),
    });
});

test("create a person's home with their personal account at sign-up, placed on a cloud machine of their residency, which a machine reads", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { machine } = await fixture.createSpace(owner);

    // read the home as a machine and find it placed on the residency's cloud machine
    const read = await machine.client(accountPackage).user.get({ id: owner.id });
    const placed = await fixture.database
        .select({
            id: placement.table.id,
            scope: placement.table.scope,
            machineId: placement.table.machineId,
            epoch: placement.table.epoch,
            targetId: placement.table.targetId,
            transferId: placement.table.transferId,
        })
        .from(placement.table)
        .where(eq(placement.table.id, read.homeId));
    expect([read.homeId, placed]).toEqual([
        Account.home(owner.account),
        [
            {
                id: Account.home(owner.account),
                scope: owner.account,
                machineId: ids.machine,
                epoch: 1,
                targetId: null,
                transferId: null,
            },
        ],
    ]);
});

test("show a person's public profile with their locale to anyone signed in, and their private fields to themselves alone", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn("person@example.com");
    const stranger = await fixture.signIn(STRANGER_EMAIL);

    // read the owner as themselves and as a stranger
    const self = await owner.client(accountPackage).user.get({ id: owner.id });
    const shown = await stranger.client(accountPackage).user.get({ id: owner.id });
    const { email, emailVerified, login, timeZone, suspendedAt, ...profile } = self;
    expect({
        private: { email, emailVerified, login, timeZone, suspendedAt },
        shown,
    }).toEqual({
        private: {
            email: "person@example.com",
            emailVerified: true,
            login: null,
            timeZone: SIGN_UP.timeZone,
            suspendedAt: null,
        },
        shown: { ...profile, name: "Person", locale: SIGN_UP.locale },
    });
});

test("refuse where a person receives notifications to another person's home and to other people", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const other = await fixture.signIn(STRANGER_EMAIL);

    // act as the other person's home, whose identity its machine started
    const home = await fixture.personalHome(other);
    await fixture.startIdentity(home.spaceId, home.machine);
    const stranger = fixture.as(home.spaceId).client(accountPackage);

    // refuse the recipient to another person's home and to the other person
    expect({
        stranger: await refusal(stranger.user.recipient({ id: owner.id })),
        other: await refusal(other.client(accountPackage).user.recipient({ id: owner.id })),
    }).toEqual({
        stranger: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: recipient"],
        other: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: recipient"],
    });
});

test("let the machine serving an account read its spaces, whose rows its replicas follow", async () => {
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId, machine } = await fixture.personalHome(owner);

    // read the home as the cloud machine serving it, as its owner reads it
    const read = await machine.client(accountPackage).space.get({ accountId, id: spaceId });
    expect(read).toEqual(await owner.client(accountPackage).space.get({ accountId, id: spaceId }));
});
