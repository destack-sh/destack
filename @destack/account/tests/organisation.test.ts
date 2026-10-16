import { accessInvitation, accessRelationship, Contact } from "@destack/access";
import { and, eq } from "@destack/db";
import { present, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { domain } from "../src/object/domain.ts";
import { group } from "../src/object/group.ts";
import type { MemberRole } from "../src/object/organisation.ts";
import { AccountFixture, type Person, TestAllowance, outcome } from "../src/test/index.ts";

test("grant an organisation's active members the access of their roles", async () => {
    // seat a person in each role below owner and suspend one more member
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const { organisationId, accountId } = await found(fixture, founder);
    const roles: readonly MemberRole[] = ["admin", "member", "billing", "guest", "member"];
    const people: Person[] = [];
    const seats: string[] = [];
    for (const [index, role] of roles.entries()) {
        const person = await fixture.person(`person-${index}@example.com`);
        people.push(person);
        seats.push(await seat(founder, person, organisationId, role));
    }
    await founder.client.member.suspend({
        organisationId,
        id: present(seats.at(-1), "the suspended seat"),
        requestId: RequestId.create(Date.now()),
    });

    // let administrators manage the account and members read it
    const accessed: string[][] = [];
    for (const person of people) {
        accessed.push(await access(person, organisationId, accountId));
    }
    // conceal the account from members outside its roles, and the whole organisation from the suspended one
    const hidden = `NOT_FOUND: no account ${accountId}`;
    const concealed = `NOT_FOUND: no scope ${organisationId}`;
    expect(accessed).toEqual([
        ["executed", "executed", "executed"],
        ["executed", "executed", `FORBIDDEN: permission denied: update`],
        ["executed", hidden, hidden],
        ["executed", hidden, hidden],
        [`NOT_FOUND: no organisation ${organisationId}`, concealed, concealed],
    ]);
});

test("change an organisation's owners only as an owner, leaving at least one", async () => {
    // seat an administrator and a member
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const administrator = await fixture.person("admin@example.com");
    const member = await fixture.person("member@example.com");
    const { organisationId } = await found(fixture, founder);
    await seat(founder, administrator, organisationId, "admin");
    const seated = await seat(founder, member, organisationId, "member");
    const founding = (await founder.client.member.list({ organisationId })).items.find(
        (entry) => entry.userId === founder.id,
    );
    const founderSeat = present(founding, "the founder's seat").id;

    // refuse the administrator promoting the member, inviting an owner and deleting the organisation
    const change = (person: Person, id: string, role: MemberRole) =>
        outcome(
            person.client.member.update({
                organisationId,
                id,
                requestId: RequestId.create(Date.now()),
                role,
            }),
        );
    const remove = (person: Person) =>
        outcome(
            person.client.organisation.delete({
                id: organisationId,
                requestId: RequestId.create(Date.now()),
            }),
        );
    const refused = [
        await change(administrator, seated, "owner"),
        await outcome(
            administrator.client.organisation.invite({
                id: organisationId,
                requestId: RequestId.create(Date.now()),
                relation: "owner",
                subject: Contact.subject(Contact.email("owner@example.com")),
            }),
        ),
        await remove(administrator),
    ];

    // refuse the elevated founder demoting the last owner
    await fixture.elevate(founder.id);
    expect([
        refused,
        await change(founder, founderSeat, "admin"),
        await change(founder, seated, "owner"),
        await change(founder, founderSeat, "admin"),
        await readOrganisation(member, organisationId),
    ]).toEqual([
        [
            `FORBIDDEN: only owners of ${organisationId} change its owners`,
            "FORBIDDEN: permission denied: own",
            "FORBIDDEN: permission denied: delete",
        ],
        `PRECONDITION_FAILED: ${organisationId} needs at least one owner`,
        "executed",
        "executed",
        "executed",
    ]);
});

test("seat the verified invitee of an invitation once, before it lapses, under the invited role", async () => {
    // invite two addresses and an invited user
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const { organisationId } = await found(fixture, founder);
    const invitee = await fixture.person("invitee@example.com");
    const late = await fixture.person("late@example.com");
    const holder = await fixture.person("holder@example.com");
    const stranger = await fixture.person("stranger@example.com");
    const invite = (subject: Subject, relation: MemberRole) =>
        founder.client.organisation.invite({
            id: organisationId,
            requestId: RequestId.create(Date.now()),
            relation,
            subject,
        });
    const invited = await invite(Contact.subject(Contact.email("Invitee@Example.com")), "member");
    const lapsing = await invite(Contact.subject(Contact.email("late@example.com")), "member");
    const holderSeat = await seat(founder, holder, organisationId, "guest");
    const promoted = await invite(holder.subject, "admin");
    const addressed = await invite(invitee.subject, "admin");

    // lapse the second invitation
    await fixture.storage.database
        .update(accessInvitation)
        .set({ expiresAt: Date.now() - 1 })
        .where(eq(accessInvitation.id, schema.identifier("invitation").parse(lapsing.id)));

    // accept as the invitee alone and once
    const accept = (person: Person, invitationId: string) =>
        outcome(
            person.client.organisation.accept({
                id: organisationId,
                invitationId,
                requestId: RequestId.create(Date.now()),
            }),
        );
    expect([
        [invited.relationship.subject, invited.inviter, invited.status],
        (await invitee.client.user.invited({ id: invitee.id })).items.map((entry) => entry.id),
        await accept(stranger, invited.id),
        await accept(invitee, invited.id),
        await accept(invitee, invited.id),
        await accept(late, lapsing.id),
        await accept(holder, promoted.id),
        await accept(invitee, addressed.id),
        (await founder.client.member.list({ organisationId })).items
            .filter((entry) => entry.userId !== founder.id)
            .map((entry) => [entry.role, entry.status]),
        await readOrganisation(invitee, organisationId),
        await readOrganisation(late, organisationId),
    ]).toEqual([
        [Contact.subject(Contact.email("invitee@example.com")), founder.subject, "pending"],
        [invited.id, addressed.id],
        "FORBIDDEN: invitation is addressed to someone else",
        "executed",
        "CONFLICT: invitation is accepted",
        "FORBIDDEN: invitation has lapsed",
        `CONFLICT: member ${holderSeat} is active`,
        `CONFLICT: member ${await seatOf(founder, invitee, organisationId)} is active`,
        [
            ["guest", "active"],
            ["member", "active"],
        ],
        "executed",
        `NOT_FOUND: no organisation ${organisationId}`,
    ]);
});

test("serve an organisation's accounts on hostnames below the domains it verified", async () => {
    // claim a domain for the organisation
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const { organisationId, accountId } = await found(fixture, founder);
    await TestAllowance.set(fixture.database, accountId, "domain", { kind: "access" });
    const claimed = await founder.client.domain.create({
        scope: organisationId,
        requestId: RequestId.create(Date.now()),
        name: "acme.com",
    });
    await fixture.storage.database
        .update(domain.table)
        .set({ verifiedAt: Date.now() })
        .where(eq(domain.table.id, claimed.id));

    // serve the organisation's account below it
    const served = await founder.client.hostname.create({
        accountId,
        requestId: RequestId.create(Date.now()),
        name: "notes.acme.com",
    });
    expect(served.name).toEqual("notes.acme.com");
});

test("make a nested group's members members of its parent, following moves and removals", async () => {
    // nest a team in one department and move it to another
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const { organisationId } = await found(fixture, founder);
    const create = (name: string, parentId?: string) =>
        founder.client.group.create({
            scope: organisationId,
            requestId: RequestId.create(Date.now()),
            name,
            ...(parentId === undefined ? {} : { parentId }),
        });
    const sales = await create("Sales");
    const support = await create("Support");
    const team = await create("Team", sales.id);
    const nested = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    await founder.client.group.move({
        scope: organisationId,
        id: team.id,
        requestId: RequestId.create(Date.now()),
        parentId: support.id,
    });
    const moved = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    await founder.client.group.delete({
        scope: organisationId,
        id: team.id,
        requestId: RequestId.create(Date.now()),
    });
    const removed = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    expect([nested, moved, removed]).toEqual([
        [[team.id], []],
        [[], [team.id]],
        [[], []],
    ]);
});

/** Create an organisation with one account as its founder, paying for it. */
async function found(fixture: AccountFixture, founder: Person) {
    // create the organisation with its account
    const organisation = await founder.client.organisation.create({
        requestId: RequestId.create(Date.now()),
        name: "Acme",
        handle: "initech",
        residencyId: "eu",
    });

    return { organisationId: organisation.id, accountId: organisation.payer };
}

/** Find a person's seat in an organisation. */
async function seatOf(founder: Person, person: Person, organisationId: string): Promise<string> {
    const { items } = await founder.client.member.list({ organisationId });

    return present(
        items.find((entry) => entry.userId === person.id),
        "the person's seat",
    ).id;
}

/** Invite a person to a seat under a role, accept as that person, and return the seat. */
async function seat(
    founder: Person,
    person: Person,
    organisationId: string,
    role: MemberRole,
): Promise<string> {
    // invite and accept
    const invitation = await founder.client.organisation.invite({
        id: organisationId,
        requestId: RequestId.create(Date.now()),
        relation: role,
        subject: person.subject,
    });
    await person.client.organisation.accept({
        id: organisationId,
        invitationId: invitation.id,
        requestId: RequestId.create(Date.now()),
    });

    return seatOf(founder, person, organisationId);
}

/** Report whether a person reads an organisation. */
function readOrganisation(person: Person, organisationId: string): Promise<string> {
    return outcome(person.client.organisation.get({ id: organisationId }));
}

/** Report whether a person reads an organisation, reads its account, and renames its account. */
async function access(person: Person, organisationId: string, accountId: string) {
    const read = () => person.client.account.get({ scope: organisationId, id: accountId });
    const rename = async () => {
        const current = await read();
        await person.client.account.update({
            scope: organisationId,
            id: accountId,
            requestId: RequestId.create(Date.now()),
            revision: current.revision,
            name: current.name,
        });
    };

    return [
        await readOrganisation(person, organisationId),
        await outcome(read()),
        await outcome(rename()),
    ];
}

/** List the groups whose members a group's member relation includes. */
async function nesting(fixture: AccountFixture, groupId: string): Promise<string[]> {
    const rows = await fixture.storage.database
        .select({ id: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.objectType, group.policy.definition.name),
                eq(accessRelationship.objectId, groupId),
                eq(accessRelationship.relation, "member"),
                eq(accessRelationship.subjectRelation, "member"),
            ),
        );

    return rows.map((row) => row.id);
}
