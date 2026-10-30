import accountPackage from "../src/package.ts";
import { accessRelationship, Contact } from "@destack/access";
import { and, eq } from "@destack/db";
import type { Environment } from "@destack/environment";
import objectPackage from "@destack/object/package";
import { present } from "@destack/schema";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import { group } from "../src/object/group.ts";
import { MEMBERSHIPS, type Membership, organization } from "../src/object/organization.ts";
import { TestAccountStore, TestDns, type TestPerson, OWNER_EMAIL } from "../src/test/index.ts";

/** Create an organization with one account as its founder, paying for it. */
async function foundOrganization(fixture: TestAccountStore, founder: TestPerson) {
    // create the organization with its account
    const created = await founder.client(accountPackage).organization.create({
        requestId: RequestId.create(fixture.store.environment),
        name: "Acme",
        handle: "initech",
        residencyId: "eu",
    });

    return { organizationId: created.id, accountId: created.payer };
}

/** List an organization's relationships through a membership relation, as a person reads them. */
async function memberships(person: TestPerson, organizationId: string) {
    const { items } = await person.client(objectPackage).relationship.list({
        scope: Scope.universe.id,
        where: { objectId: organizationId, relation: { in: [...MEMBERSHIPS] } },
    });

    return items;
}

/** Revoke a membership of an organization as a person. */
async function leave(person: TestPerson, id: string, environment: Environment): Promise<void> {
    await person.client(objectPackage).relationship.delete({
        scope: Scope.universe.id,
        id,
        requestId: RequestId.create(environment),
    });
}

/** Report whether a person reads an organization. */
function readOrganization(person: TestPerson, organizationId: string): Promise<Refusal> {
    return refusal(person.client(accountPackage).organization.get({ id: organizationId }));
}

/** Report whether a person reads an organization, reads its account, and renames its account. */
async function access(
    person: TestPerson,
    organizationId: string,
    accountId: string,
    environment: Environment,
) {
    const read = () =>
        person.client(accountPackage).account.get({ scope: organizationId, id: accountId });
    const rename = async () => {
        const current = await read();
        await person.client(accountPackage).account.update({
            scope: organizationId,
            id: accountId,
            requestId: RequestId.create(environment),
            revision: current.revision,
            name: current.name,
        });
    };

    return [
        await readOrganization(person, organizationId),
        await refusal(read()),
        await refusal(rename()),
    ];
}

/** List the groups whose members a group's member relation includes. */
async function nesting(fixture: TestAccountStore, groupId: string): Promise<string[]> {
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

test("grant an organization's members the access of their relations", async () => {
    // add a person in each relation below owner, and revoke one more person's membership
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");
    const { organizationId, accountId } = await foundOrganization(fixture, founder);
    const relations: (readonly [string, Membership])[] = [
        ["administrator@example.com", "admin"],
        ["member@example.com", "member"],
        ["partner@example.com", "billing"],
        ["assistant@example.com", "guest"],
        ["person@example.com", "member"],
    ];
    const people: TestPerson[] = [];
    const joined: string[] = [];
    for (const [email, relation] of relations) {
        const person = await fixture.signIn(email);
        people.push(person);
        joined.push((await fixture.join(founder, person, organizationId, relation)).id);
    }
    await leave(
        founder,
        present(joined.at(-1), "the revoked membership"),
        fixture.store.environment,
    );

    // let administrators manage the account and members read it
    const accessed: Refusal[][] = [];
    for (const person of people) {
        accessed.push(await access(person, organizationId, accountId, fixture.store.environment));
    }

    // conceal the account from people outside it, and the whole organization from the one who left
    const hidden = ["NOT_FOUND", "NOT_FOUND", `account ${accountId} not found`];
    const concealed = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${organizationId} not found`];
    expect(accessed).toEqual([
        ["done", "done", "done"],
        ["done", "done", ["FORBIDDEN", "PERMISSION_DENIED", `permission denied: update`]],
        ["done", hidden, hidden],
        ["done", hidden, hidden],
        [
            ["NOT_FOUND", "NOT_FOUND", `organization ${organizationId} not found`],
            concealed,
            concealed,
        ],
    ]);
});

test("change an organization's owners only as an owner, keeping at least one", async () => {
    // add an administrator and a member
    await using fixture = await TestAccountStore.open();
    let founder = await fixture.signIn("founder@example.com");
    const administrator = await fixture.signIn("administrator@example.com");
    const member = await fixture.signIn("member@example.com");
    const { organizationId } = await foundOrganization(fixture, founder);
    await fixture.join(founder, administrator, organizationId, "admin");
    await fixture.join(founder, member, organizationId, "member");
    const founding = present(
        (await memberships(founder, organizationId)).find(
            (entry) => entry.relation === "owner" && entry.subjectId === founder.id,
        ),
        "the founder's membership",
    );

    // refuse the administrator making the member an owner, inviting an owner and deleting the organization
    const own = (person: TestPerson, subject: Subject) =>
        person.client(objectPackage).relationship.create({
            scope: Scope.universe.id,
            requestId: RequestId.create(fixture.store.environment),
            object: organization.reference(Scope.universe.id, organizationId),
            relation: "owner",
            subject,
        });
    const refused = [
        await refusal(own(administrator, member.subject)),
        await refusal(
            administrator.client(objectPackage).invitation.create({
                scope: Scope.universe.id,
                requestId: RequestId.create(fixture.store.environment),
                object: organization.reference(Scope.universe.id, organizationId),
                relationship: {
                    relation: "owner",
                    subject: Contact.subject(Contact.email(OWNER_EMAIL)),
                },
            }),
        ),
        await refusal(
            administrator.client(accountPackage).organization.delete({
                id: organizationId,
                requestId: RequestId.create(fixture.store.environment),
            }),
        ),
    ];

    // refuse the elevated founder leaving as the last owner, then hand ownership to the member
    founder = await fixture.elevate(founder);
    expect([
        refused,
        await refusal(leave(founder, founding.id, fixture.store.environment)),
        await refusal(own(founder, member.subject)),
        await refusal(leave(founder, founding.id, fixture.store.environment)),
        await readOrganization(member, organizationId),
    ]).toEqual([
        [
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: administer"],
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: administer"],
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: delete"],
        ],
        [
            "CONFLICT",
            "LAST_RELATIONSHIP",
            `the last owner of organization ${organizationId} cannot be removed`,
        ],
        "done",
        "done",
        "done",
    ]);
});

test("list an invitation to a person's email to them, and make them a member once they accept it", async () => {
    // invite a signed-up person's email to the organization
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");
    const { organizationId } = await foundOrganization(fixture, founder);
    const invitee = await fixture.signIn("person@example.com");
    const object = organization.reference(Scope.universe.id, organizationId);
    const invited = await founder.client(objectPackage).invitation.create({
        scope: Scope.universe.id,
        requestId: RequestId.create(fixture.environment),
        object,
        relationship: {
            relation: "member",
            subject: Contact.subject(Contact.email(invitee.email)),
        },
    });

    // list it to the invitee, and accept it as them
    const listed = await invitee.client(accountPackage).user.invited({ id: invitee.id });
    await invitee.client(objectPackage).invitation.accept({
        scope: Scope.universe.id,
        requestId: RequestId.create(fixture.environment),
        object,
        invitationId: invited.id,
    });

    // add the invitee as a member, who reads the organization from then on
    const members = (await memberships(founder, organizationId))
        .filter((entry) => entry.subjectId !== founder.id)
        .map((entry) => [entry.relation, entry.subjectId]);
    expect([
        listed.items.map((entry) => entry.id),
        members,
        await readOrganization(invitee, organizationId),
    ]).toEqual([[invited.id], [["member", invitee.id]], "done"]);
});

test("serve an organization's accounts on hostnames below the domains it verified", async () => {
    // claim a domain for the organization over a test DNS
    const dns = new TestDns();
    await using fixture = await TestAccountStore.open({ resolver: dns });
    const founder = await fixture.signIn("founder@example.com");
    const { organizationId, accountId } = await foundOrganization(fixture, founder);
    await fixture.entitle(accountId, "domain", { kind: "boolean" });
    const claimed = await founder.client(accountPackage).domain.create({
        scope: organizationId,
        requestId: RequestId.create(fixture.store.environment),
        name: "acme.com",
    });

    // verify it through its published challenge record
    dns.publish(claimed);
    await founder.client(accountPackage).domain.verify({
        scope: organizationId,
        id: claimed.id,
        requestId: RequestId.create(fixture.store.environment),
    });

    // serve the organization's account below it
    const served = await founder.client(accountPackage).hostname.create({
        accountId,
        requestId: RequestId.create(fixture.store.environment),
        name: "notes.acme.com",
    });
    expect(served).toEqual({
        id: served.id,
        scope: accountId,
        name: "notes.acme.com",
        revision: 1,
        tags: {},
        createdAt: fixture.clock.now(),
        ...ObjectReference.columns("createdBy", founder.subject),
        updatedAt: fixture.clock.now(),
        ...ObjectReference.columns("updatedBy", founder.subject),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
    });
});

test("make a nested group's members members of its parent, following moves and removals", async () => {
    // nest a team in one department and move it to another
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");
    const { organizationId } = await foundOrganization(fixture, founder);
    const create = (name: string, parentId?: string) =>
        founder.client(accountPackage).group.create({
            scope: organizationId,
            requestId: RequestId.create(fixture.store.environment),
            name,
            ...(parentId === undefined ? {} : { parentId }),
        });
    const sales = await create("Sales");
    const support = await create("Support");
    const team = await create("Team", sales.id);
    const nested = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    await founder.client(accountPackage).group.move({
        scope: organizationId,
        id: team.id,
        requestId: RequestId.create(fixture.store.environment),
        parentId: support.id,
    });
    const moved = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    await founder.client(accountPackage).group.delete({
        scope: organizationId,
        id: team.id,
        requestId: RequestId.create(fixture.store.environment),
    });
    const removed = [await nesting(fixture, sales.id), await nesting(fixture, support.id)];
    expect([nested, moved, removed]).toEqual([
        [[team.id], []],
        [[], [team.id]],
        [[], []],
    ]);
});
