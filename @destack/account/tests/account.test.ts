import { session } from "../src/object/session.ts";
import { Subject, ObjectReference } from "@destack/sync";
import { Account, account } from "../src/object/account.ts";
import { expect, refusal, test } from "@destack/test";
import { RequestId } from "@destack/runtime/request";
import { eq, Snapshot } from "@destack/db";
import accountPackage from "../src/package.ts";
import { SESSION_TOKEN_TYPE } from "../src/oauth/index.ts";
import { placement } from "../src/object/placement.ts";
import { AccountFixture, ids, type Person, SIGN_UP, outcome } from "../src/test/index.ts";
import { claims } from "./fixture.ts";
import { aligned, Identifier, schema } from "@destack/schema";
import { principal } from "@destack/access";

test("create a person's shared accounts beside their personal one, refuse a second personal one, hide them from others, and refuse exchanging a revoked session", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.storage.database;
    const owner = await fixture.person("owner@example.com");
    const client = owner.client;

    // rename the user
    const person = await client.user.get({ id: owner.id });
    const renamed = await client.user.update({
        id: owner.id,
        requestId: RequestId.create(Date.now()),
        revision: person.revision,
        name: "Florian",
    });

    // create a shared account beside the personal one, and refuse a second personal one
    const creation = {
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        handle: "florian",
        name: "Personal",
        residencyId: "eu" as const,
        kind: "shared" as const,
    };
    const shared = await client.account.create(creation);
    const personal = await outcome(
        client.account.create({
            ...creation,
            requestId: RequestId.create(Date.now()),
            handle: "second",
            kind: "personal",
        }),
    );
    const { items } = await client.account.list({ scope: owner.id, limit: 10 });
    expect({
        name: renamed.name,
        shared,
        personal,
        listed: items.map((item) => [item.handle, item.kind]),
    }).toEqual({
        name: "Florian",
        shared: {
            id: shared.id,
            createdAt: shared.createdAt,
            ...ObjectReference.columns("createdBy", owner.subject),
            updatedAt: shared.createdAt,
            ...ObjectReference.columns("updatedBy", owner.subject),
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
        personal: "CONFLICT: another account has the same scope",
        listed: [
            [owner.handle, "personal"],
            ["florian", "shared"],
        ],
    });

    // show the owner's public profile to another signed-in user, and hide their accounts
    const update = {
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        id: shared.id,
        revision: shared.revision,
        name: "Home",
    };
    const outsider = await fixture.person("other@example.com");
    const hidden = await Promise.all(
        [
            outsider.client.user.get({ id: owner.id }),
            outsider.client.account.get({ scope: owner.id, id: shared.id }),
            outsider.client.account.update(update),
            outsider.client.account.create({
                ...creation,
                requestId: RequestId.create(Date.now()),
                handle: "intruder",
            }),
        ].map((attempt) => outcome(attempt)),
    );
    const listed = await outsider.client.account.list({ scope: owner.id, limit: 10 });
    expect({ hidden, listed: listed.items }).toEqual({
        hidden: [
            "executed",
            `NOT_FOUND: no account ${shared.id}`,
            `NOT_FOUND: no account ${shared.id}`,
            "NOT_FOUND: account not found",
        ],
        listed: [],
    });

    // refuse exchanging the owner's session once it is revoked
    await database
        .update(session.table)
        .set({ revokedAt: Date.now() })
        .where(eq(session.table.userId, owner.id));
    expect(
        await outcome(
            fixture.exchangeToken(owner.session, SESSION_TOKEN_TYPE, {
                audience: accountPackage.id,
            }),
        ),
    ).toBe("BAD_REQUEST: invalid_grant: session is no longer authorized");
});

/** Act as another user only with authority it lent or as an elevated administrator. */
test("exchange tokens acting as a user through lent authority or account administration", async () => {
    await using fixture = await AccountFixture.open();
    const administrator = await fixture.person("administrator@example.com");
    const member = await fixture.person("member@example.com");
    const assistant = await fixture.person("assistant@example.com");

    // create an account with a space and make the member belong to it
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    await administrator.client.account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(Date.now()),
        relation: "editor",
        subject: member.subject,
    });

    // read who a token represents and who acts in it
    const exchange = async (acting: Person, space = spaceId) => {
        const token = await fixture.exchangeToken(acting.session, SESSION_TOKEN_TYPE, {
            audience: accountPackage.id,
            space,
            subject: Subject.key(member.subject),
        });
        const claimed = claims(token.accessToken);

        return [claimed.caller.subject, claimed.act];
    };
    const acting = (who: Person) => [member.subject, [who.subject]];

    // let the administrator act as the member only once elevated
    const refused = "BAD_REQUEST: invalid_grant: acting as another user is not allowed";
    expect(await outcome(exchange(administrator))).toBe(refused);
    await fixture.elevate(administrator.id);
    expect(await exchange(administrator)).toEqual(acting(administrator));

    // refuse the administrator acting as the member in another account's space
    const elsewhere = await fixture.createSpace(administrator);
    expect(await outcome(exchange(administrator, elsewhere.spaceId))).toBe(refused);

    // let the assistant act as the member only with lent authority and elevation
    expect(await outcome(exchange(assistant))).toBe(refused);
    const lending = {
        id: member.id,
        requestId: RequestId.create(Date.now()),
        relation: "delegate",
        subject: assistant.subject,
        expiresAt: Date.now() + 60_000,
    };
    expect(await outcome(member.client.user.grant(lending))).toBe(
        "INSUFFICIENT_AUTHENTICATION: authentication assurance is too low: authenticate again",
    );
    await fixture.elevate(member.id);
    const lent = await member.client.user.grant(lending);
    await fixture.elevate(assistant.id);
    expect(await exchange(assistant)).toEqual(acting(assistant));
    await member.client.user.revoke({
        id: member.id,
        requestId: RequestId.create(Date.now()),
        relationshipId: schema.identifier("relationship").parse(lent.id),
    });
    expect(await outcome(exchange(assistant))).toBe(refused);
});

/** Describe an impersonation in a space as the member, with its memberships and the acting user. */
test("exchange a token acting as a member in the account's space, naming the member's memberships and the acting administrator", async () => {
    await using fixture = await AccountFixture.open();
    const administrator = await fixture.person("administrator@example.com");
    const member = await fixture.person("member@example.com");

    // create an account's space and make the member belong to it
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    await administrator.client.account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(Date.now()),
        relation: "editor",
        subject: member.subject,
    });
    await fixture.elevate(administrator.id);

    // exchange the administrator's session for a token acting as the member in the space
    const acting = await fixture.exchangeToken(administrator.session, SESSION_TOKEN_TYPE, {
        audience: accountPackage.id,
        space: spaceId,
        subject: Subject.key(member.subject),
    });
    const sessions = await fixture.storage.database
        .select({ id: session.table.id, authenticatedAt: session.table.authenticatedAt })
        .from(session.table)
        .where(eq(session.table.userId, administrator.id));
    const signedIn = aligned(sessions, 0);

    // name the member, their memberships, the administrator's assurance and the administrator acting
    const claimed = claims(acting.accessToken);
    expect([claimed.space, claimed.act, claimed.caller]).toEqual([
        spaceId,
        [administrator.subject],
        {
            subject: member.subject,
            groups: [
                { ...account.reference(administrator.id, accountId), relation: "editor" },
                { ...account.reference(administrator.id, accountId), relation: "member" },
            ],
            contacts: [],
            credential: {
                kind: "session",
                id: signedIn.id,
                assurance: { level: 2, authenticatedAt: signedIn.authenticatedAt },
            },
        },
    ]);
});

test("resolve the paying account: an organisation's payer for it and its accounts, and a person's account itself", async () => {
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");

    // create an organisation paying through its own account, with a second account in it
    const acme = await founder.client.organisation.create({
        requestId: RequestId.create(Date.now()),
        name: "Acme",
        handle: "acme-pays",
        residencyId: "us",
    });
    const second = await founder.client.account.create({
        scope: acme.id,
        requestId: RequestId.create(Date.now()),
        handle: "acme-second",
        name: "Acme",
        kind: "shared",
    });

    // charge the organisation and both its accounts to its payer, and the person's account to itself
    const snapshot = Snapshot.live(fixture.database);
    expect([
        await Account.payer(snapshot, acme.id),
        await Account.payer(snapshot, second.id),
        await Account.payer(snapshot, acme.payer),
        await Account.payer(snapshot, founder.account),
    ]).toEqual([acme.payer, acme.payer, acme.payer, founder.account]);
});

/** Root an organisation's accounts at its owners, with the organisation's residency. */
test("create accounts in an organisation, administered by its owner members", async () => {
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const partner = await fixture.person("partner@example.com");

    // create an organisation with its own account, and another account in it
    const acme = await founder.client.organisation.create({
        requestId: RequestId.create(Date.now()),
        name: "Acme",
        handle: "acme-inc",
        residencyId: "us",
    });
    const created = await founder.client.account.create({
        scope: acme.id,
        requestId: RequestId.create(Date.now()),
        handle: "initech",
        name: "Acme",
        kind: "shared",
    });
    expect(created).toEqual({
        id: created.id,
        createdAt: created.createdAt,
        ...ObjectReference.columns("createdBy", founder.subject),
        updatedAt: created.createdAt,
        ...ObjectReference.columns("updatedBy", founder.subject),
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

    // hide the account from the partner until the partner has an owner seat
    const rename = (person: Person) =>
        person.client.account.update({
            scope: acme.id,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            revision: created.revision,
            name: "Acme Inc",
        });
    expect(await outcome(rename(partner))).toBe(`NOT_FOUND: no scope ${acme.id}`);
    await fixture.elevate(founder.id);
    await founder.client.organisation.update({
        id: acme.id,
        requestId: RequestId.create(Date.now()),
        payer: created.id,
    });
    const invitation = await founder.client.organisation.invite({
        id: acme.id,
        requestId: RequestId.create(Date.now()),
        relation: "owner",
        subject: partner.subject,
    });
    await partner.client.organisation.accept({
        id: acme.id,
        invitationId: invitation.id,
        requestId: RequestId.create(Date.now()),
    });
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
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { machine } = await fixture.createSpace(owner);

    // read the home as a machine and find it placed on the residency's cloud machine
    const read = await machine.client.user.home({ id: owner.id });
    const placed = await Snapshot.live(fixture.database)
        .read(placement.table)
        .findFirst({ where: { id: read } });
    expect([
        read,
        placed && {
            id: placed.id,
            scope: placed.scope,
            machineId: placed.machineId,
            epoch: placed.epoch,
            targetId: placed.targetId,
            transferId: placed.transferId,
        },
    ]).toEqual([
        Account.home(owner.account),
        {
            id: Account.home(owner.account),
            scope: owner.account,
            machineId: ids.machine,
            epoch: 1,
            targetId: null,
            transferId: null,
        },
    ]);
});

test("show a person's public profile with their locale to anyone signed in, and their private fields to themselves alone", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const stranger = await fixture.person("stranger@example.com");

    // read the owner as themselves and as a stranger
    const own = await owner.client.user.get({ id: owner.id });
    const shown = await stranger.client.user.get({ id: owner.id });
    expect({
        own: [own.name, own.email, own.locale],
        shown: [shown.name, shown.email, shown.locale, shown.timeZone],
    }).toEqual({
        own: ["owner@example.com", "owner@example.com", SIGN_UP.locale],
        shown: ["owner@example.com", undefined, SIGN_UP.locale, undefined],
    });
});

test("refuse where a person receives notifications to another home's installations and to other people", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const other = await fixture.person("other@example.com");

    // act as an installation of the other person's home
    const stranger = fixture.installation(
        principal.installation.reference(
            Account.home(other.account),
            Identifier.create("installation"),
        ),
    );

    // refuse the recipient to another home's installation and to the other person
    expect({
        stranger: await refusal(stranger.user.recipient({ id: owner.id })),
        other: await refusal(other.client.user.recipient({ id: owner.id })),
    }).toEqual({
        stranger: ["FORBIDDEN", "permission denied: recipient"],
        other: ["FORBIDDEN", "permission denied: recipient"],
    });
});

test("let the machine serving an account read its spaces, whose rows its replicas follow", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, spaceId, machine } = await fixture.personalHome(owner);

    // read the home as the cloud machine serving it
    const read = await machine.client.space.get({ accountId, id: spaceId });
    expect([read.id, read.scope]).toEqual([spaceId, accountId]);
});
