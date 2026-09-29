import { session } from "../src/object/authentication.ts";
import { account } from "../src/object/account.ts";
import { expect, test } from "@destack/test";
import { RequestId } from "@destack/service/request";
import { eq } from "@destack/db";
import { accountPackage } from "../src/audit/index.ts";
import { accountJournal } from "../src/stack/index.ts";
import { AccountFixture, claims, outcome, type Person } from "./fixture.ts";
import { identifier } from "@destack/schema";

/** Commit authorized account changes, idempotent responses, and audit events through HTTP. */
test("administer accounts with sessions and transactional audit", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.opened.database;
    const owner = await fixture.signIn("owner@example.com");
    const client = owner.client;

    // rename the user and choose a residency once, replaying the same request
    const person = await client.user.get({ id: owner.id });
    const profile = {
        id: owner.id,
        requestId: RequestId.create(),
        revision: person.revision,
        name: "Florian",
        residency: "eu" as const,
    };
    const renamed = await client.user.update(profile);
    expect(renamed).toEqual({
        ...person,
        name: "Florian",
        residency: "eu",
        revision: 2,
        updatedAt: renamed.updatedAt,
    });
    expect(await client.user.update(profile)).toEqual(renamed);

    // create two independently rooted accounts and paginate their stable identifiers
    const creation = {
        scope: owner.id,
        requestId: RequestId.create(),
        handle: "florian",
        name: "Personal",
        defaultResidency: "eu" as const,
        kind: "personal" as const,
    };
    const first = await client.account.create(creation);
    expect(first).toEqual({
        id: first.id,
        createdAt: first.createdAt,
        updatedAt: first.createdAt,
        revision: 1,
        tags: {},
        handle: "florian",
        name: "Personal",
        defaultResidency: "eu",
        kind: "personal",
        packagePolicyId: null,
        packagePolicyRegion: null,
        networkPolicyId: null,
        networkPolicyRegion: null,
        deletionRequestedAt: null,
        scope: owner.id,
    });
    expect(await client.account.create(creation)).toEqual(first);
    const second = await client.account.create({
        ...creation,
        requestId: RequestId.create(),
        handle: "work",
        kind: "shared",
    });
    const page = await client.account.list({ scope: owner.id, limit: 1 });
    expect(page.items).toEqual([first]);
    expect(await client.account.list({ scope: owner.id, limit: 1, cursor: page.cursor! })).toEqual({
        items: [second],
        cursor: null,
    });

    // replay the same mutation, and reject stale revisions, mismatched request contents and taken handles
    const update = {
        scope: owner.id,
        requestId: RequestId.create(),
        id: first.id,
        revision: first.revision,
        name: "Home",
    };
    const changed = await client.account.update(update);
    expect(changed).toEqual({ ...first, name: "Home", revision: 2, updatedAt: changed.updatedAt });
    expect(await client.account.update(update)).toEqual(changed);
    const failures = await Promise.all(
        [
            client.account.update({ ...update, name: "Wrong" }),
            client.account.update({ ...update, requestId: RequestId.create() }),
            client.account.create({ ...creation, requestId: RequestId.create() }),
        ].map((attempt) => outcome(attempt)),
    );
    expect(failures).toEqual([
        "CONFLICT: request identifier has already been used",
        "CONFLICT: account revision has changed",
        "CONFLICT: a record with the same unique key exists",
    ]);
    expect(await client.account.get({ scope: owner.id, id: first.id })).toEqual(changed);

    // hide the owner and their accounts from another signed-in user, who creates none in the owner's scope
    const outsider = await fixture.signIn("other@example.com");
    const hidden = await Promise.all(
        [
            outsider.client.user.get({ id: owner.id }),
            outsider.client.account.list({ scope: owner.id, limit: 10 }),
            outsider.client.account.get({ scope: owner.id, id: first.id }),
            outsider.client.account.update(update),
            outsider.client.account.create({
                ...creation,
                requestId: RequestId.create(),
                handle: "intruder",
                kind: "shared",
            }),
        ].map((attempt) => outcome(attempt)),
    );
    expect(hidden).toEqual([
        `NOT_FOUND: no user ${owner.id}`,
        `NOT_FOUND: no scope ${owner.id}`,
        `NOT_FOUND: no scope ${owner.id}`,
        `NOT_FOUND: no scope ${owner.id}`,
        `NOT_FOUND: no scope ${owner.id}`,
    ]);

    // journal the executed requests and the final failures, auditing the executed ones and the failed calls
    const entries = await database.select({ outcome: accountJournal.outcome }).from(accountJournal);
    expect(
        entries
            .map((entry) =>
                entry.outcome === null || "value" in entry.outcome
                    ? "executed"
                    : entry.outcome.error.code,
            )
            .sort(),
    ).toEqual([
        "CONFLICT",
        "CONFLICT",
        "NOT_FOUND",
        "NOT_FOUND",
        "executed",
        "executed",
        "executed",
        "executed",
    ]);
    const events = await fixture.delivered();
    const actor = { type: "subject", subject: owner.subject };
    const success = { stage: "result", outcome: "success" };
    const conflict = { stage: "result", outcome: "failure", errorCode: "CONFLICT" };
    const audited = events
        .filter((event) =>
            ["User.update", "Account.create", "Account.update"].includes(event.action.name),
        )
        .map((event) => ({
            action: event.action.name,
            scope: event.context.scope,
            actor: event.context.actor,
            details: event.details,
            result: event.result,
        }));

    // order the concurrent failures by action
    const key = (event: (typeof audited)[number]) =>
        `${event.action} ${"outcome" in event.result ? event.result.outcome : event.result.stage}`;
    expect(audited.toSorted((left, right) => key(left).localeCompare(key(right)))).toEqual([
        { action: "Account.create", scope: owner.id, actor, details: {}, result: conflict },
        { action: "Account.create", scope: owner.id, actor, details: {}, result: success },
        { action: "Account.create", scope: owner.id, actor, details: {}, result: success },
        { action: "Account.update", scope: owner.id, actor, details: {}, result: conflict },
        { action: "Account.update", scope: owner.id, actor, details: {}, result: success },
        { action: "User.update", scope: "universe", actor, details: {}, result: success },
    ]);

    // revoke the session before a replay and preserve the previously committed account
    await database
        .update(session.table)
        .set({ revokedAt: Date.now() })
        .where(eq(session.table.userId, owner.id));
    await expect(
        client.account.update({ ...update, requestId: RequestId.create() }),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "the person is not signed in" });
    expect(
        await database
            .select({ name: account.table.name, revision: account.table.revision })
            .from(account.table)
            .where(eq(account.table.id, first.id)),
    ).toEqual([{ name: "Home", revision: 2 }]);
});

/** Act as another user only with authority it lent or as an elevated administrator of its account. */
test("exchange tokens acting as a user through lent authority or account administration", async () => {
    await using fixture = await AccountFixture.open();
    const administrator = await fixture.signIn("administrator@example.com");
    const member = await fixture.signIn("member@example.com");
    const helper = await fixture.signIn("helper@example.com");

    // create an account with a space, and make the member belong to it
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    await administrator.client.account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(),
        relation: "member",
        subject: member.subject,
    });

    // read who a token represents and who acts in it
    const exchange = async (acting: Person) => {
        const token = await acting.client.authentication.exchange({
            audience: accountPackage.id,
            spaceId,
            subject: member.subject,
        });
        const caller = claims(token.accessToken);

        return [caller.subject, caller.delegates];
    };
    const acting = (who: Person) => [member.subject, [{ subject: who.subject, authority: "full" }]];

    // let the administrator act as the member only once elevated
    await expect(exchange(administrator)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "acting as another user is not allowed",
    });
    await fixture.elevate(administrator);
    expect(await exchange(administrator)).toEqual(acting(administrator));

    // let the helper act as the member only with lent authority and elevation
    await expect(exchange(helper)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "acting as another user is not allowed",
    });
    const lending = {
        id: member.id,
        requestId: RequestId.create(),
        relation: "delegate",
        subject: helper.subject,
        expiresAt: Date.now() + 60_000,
    };
    await expect(member.client.user.grant(lending)).rejects.toMatchObject({
        code: "INSUFFICIENT_AUTHENTICATION",
        message: "authenticate again at the required assurance",
    });
    await fixture.elevate(member);
    const lent = await member.client.user.grant(lending);
    await fixture.elevate(helper);
    expect(await exchange(helper)).toEqual(acting(helper));
    await member.client.user.revoke({
        id: member.id,
        requestId: RequestId.create(),
        relationshipId: identifier("relationship").parse(lent.id),
    });
    await expect(exchange(helper)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "acting as another user is not allowed",
    });
});

/** Verify exchanged tokens only in the verifying account's spaces, describing an impersonation as the represented user. */
test("verify tokens in the account's spaces, as the user an impersonation represents", async () => {
    await using fixture = await AccountFixture.open();
    const administrator = await fixture.signIn("administrator@example.com");
    const member = await fixture.signIn("member@example.com");

    // create an account with a space, a space of another account, and make the member belong to the first
    const { accountId, spaceId } = await fixture.createSpace(administrator);
    const elsewhere = await fixture.createSpace(administrator);
    await administrator.client.account.grant({
        scope: administrator.id,
        id: accountId,
        requestId: RequestId.create(),
        relation: "member",
        subject: member.subject,
    });
    await fixture.elevate(administrator);

    // verify the administrator's token acting as the member, as the member with the administrator acting
    const acting = await administrator.client.authentication.exchange({
        audience: accountPackage.id,
        spaceId,
        subject: member.subject,
    });
    const verified = await administrator.client.authentication.verify({
        accountId,
        audience: accountPackage.id,
        spaceId,
        token: acting.accessToken,
    });
    const [signedIn] = await fixture.opened.database
        .select({ id: session.table.id, authenticatedAt: session.table.authenticatedAt })
        .from(session.table)
        .where(eq(session.table.userId, administrator.id));

    // take the verification time from the answer, since the service reads its own clock
    expect(verified).toEqual({
        scope: spaceId,
        audience: accountPackage.id,
        verifiedAt: verified.verifiedAt,
        expiresAt: acting.expiresAt,
        credential: { kind: "session", id: signedIn!.id },
        subject: member.subject,
        subjects: [
            member.subject,
            { ...account.reference(administrator.id, accountId), relation: "member" },
        ],
        assurance: { level: 2, authenticatedAt: signedIn!.authenticatedAt },
        delegates: [{ subject: administrator.subject, authority: "full" }],
    });

    // refuse verifying a token for a space of another account
    const outside = await administrator.client.authentication.exchange({
        audience: accountPackage.id,
        spaceId: elsewhere.spaceId,
    });
    await expect(
        administrator.client.authentication.verify({
            accountId,
            audience: accountPackage.id,
            spaceId: elsewhere.spaceId,
            token: outside.accessToken,
        }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: `${elsewhere.spaceId} is not a space of ${accountId}`,
    });
});

/** Root an organisation's accounts at its owners, with the organisation's residency. */
test("create accounts in an organisation, administered by its owners", async () => {
    await using fixture = await AccountFixture.open();
    const founder = await fixture.signIn("founder@example.com");
    const partner = await fixture.signIn("partner@example.com");

    // create an organisation and an account in it
    const acme = await founder.client.organisation.create({
        requestId: RequestId.create(),
        name: "Acme",
        residency: "us",
    });
    const created = await founder.client.account.create({
        scope: acme.id,
        requestId: RequestId.create(),
        handle: "acme",
        name: "Acme",
        kind: "shared",
    });
    expect(created).toEqual({
        id: created.id,
        createdAt: created.createdAt,
        updatedAt: created.createdAt,
        revision: 1,
        tags: {},
        handle: "acme",
        name: "Acme",
        defaultResidency: "us",
        kind: "shared",
        packagePolicyId: null,
        packagePolicyRegion: null,
        networkPolicyId: null,
        networkPolicyRegion: null,
        deletionRequestedAt: null,
        scope: acme.id,
    });

    // hide the account from the partner until the founder makes the partner an owner
    const rename = (person: Person) =>
        person.client.account.update({
            scope: acme.id,
            id: created.id,
            requestId: RequestId.create(),
            revision: created.revision,
            name: "Acme Inc",
        });
    expect(await outcome(rename(partner))).toBe(`NOT_FOUND: no scope ${acme.id}`);
    await fixture.elevate(founder);
    await founder.client.organisation.grant({
        id: acme.id,
        requestId: RequestId.create(),
        relation: "owner",
        subject: partner.subject,
    });
    const renamed = await rename(partner);
    expect(renamed).toEqual({
        ...created,
        name: "Acme Inc",
        revision: 2,
        updatedAt: renamed.updatedAt,
    });
});
