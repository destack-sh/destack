import { AccessContext, Caller, principal } from "@destack/access";
import { present } from "@destack/schema";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { approval, type Approval, changeRequest } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import {
    agentOf,
    alice,
    type ApprovalFixture,
    bob,
    carol,
    note,
    notesPackage,
    openApprovals,
} from "./fixture/approval.ts";

/** Open a change as a requester, answering its identifier. */
async function openChange(
    fixture: ApprovalFixture,
    requester: Subject,
    input: {
        readonly id: string;
        readonly call: ReturnType<typeof note.call>;
        readonly approvers?: readonly Subject[];
        readonly count?: number;
        readonly as?: "requester" | "approver";
    },
): Promise<string> {
    const { id, call, approvers, count, as } = input;
    const opened = await fixture.served
        .as(requester)
        .client(objectPackage)
        .changeRequest.create({
            scope: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            parent: { packageId: note.package.id, type: note.name, id },
            call,
            ...(approvers === undefined ? {} : { approvers: [...approvers] }),
            ...(count === undefined ? {} : { count }),
            ...(as === undefined ? {} : { as }),
        });

    return opened.id;
}

/** Run the world until an approver is asked on a change, answering the approval. */
async function asked(
    fixture: ApprovalFixture,
    change: string,
    approver: Subject,
): Promise<Approval> {
    const find = async () =>
        (await fixture.served.database.select().from(approval.table)).find(
            (row) =>
                row.parentId === change && ObjectReference.at("approver", row).id === approver.id,
        );
    await fixture.when(async () => (await find()) !== undefined);

    return present(await find(), `the approval ${approver.id} was asked for`);
}

/** Run the world until a change leaves the open state, answering its state and failure reason. */
async function settled(
    fixture: ApprovalFixture,
    change: string,
): Promise<readonly [string, string | null]> {
    const find = async () =>
        (await fixture.served.database.select().from(changeRequest.table)).find(
            (row) => row.id === change && row.state !== "open",
        );
    await fixture.when(async () => (await find()) !== undefined);
    const row = present(await find(), `the settled change ${change}`);

    return [row.state, row.failure?.reason ?? null];
}

/** Read the note's title. */
async function title(fixture: ApprovalFixture): Promise<string | undefined> {
    const [row] = await fixture.served.database.select().from(note.table);

    return row?.title;
}

test("rename a note as its requester once bob approves the change", async () => {
    // open the change as alice, asking bob, and approve it as bob
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const rename = note.call("update", { scope, id, title: "Final" });
    const change = await openChange(fixture, alice, { id, call: rename, approvers: [bob] });
    const before = await title(fixture);
    await fixture.decide(bob, await asked(fixture, change, bob), "approve");

    // apply it as alice
    expect([before, await settled(fixture, change), await title(fixture)]).toEqual([
        "Plan",
        ["applied", null],
        "Final",
    ]);
});

test("record the refusal an approved change's call answers", async () => {
    // approve a change renaming a missing note
    const fixture = await openApprovals();
    const id = await fixture.write();
    const missing = "note-01996ab0-0000-7000-8000-0000000000ff";
    const rename = note.call("update", {
        scope: fixture.served.scope,
        id: missing,
        title: "Final",
    });
    const change = await openChange(fixture, alice, { id, call: rename, approvers: [bob] });
    await fixture.decide(bob, await asked(fixture, change, bob), "approve");

    // fail the change with the update's refusal
    expect(await settled(fixture, change)).toEqual(["failed", "NOT_FOUND"]);
});

test("withdraw a change before its approvers decide, leaving it unapplied once they approve", async () => {
    // open a change, withdraw it, then approve it as bob
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const rename = note.call("update", { scope, id, title: "Final" });
    const change = await openChange(fixture, alice, { id, call: rename, approvers: [bob] });
    const pending = await asked(fixture, change, bob);
    await fixture.served
        .as(alice)
        .client(objectPackage)
        .changeRequest.withdraw({
            scope,
            requestId: RequestId.create(fixture.environment),
            id: change,
        });
    await fixture.decide(bob, pending, "approve");

    // keep the change withdrawn and the note as it was
    expect([await settled(fixture, change), await title(fixture)]).toEqual([
        ["withdrawn", null],
        "Plan",
    ]);
});

test("publish alice's note as alice once she approves bob's elevated change, and fail the same change as bob", async () => {
    // ask alice to publish her note as the approver, then as bob the requester
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const ask = async (as: "requester" | "approver") => {
        const publish = note.call("publish", { scope, id, title: as });
        const change = await openChange(fixture, bob, {
            id,
            call: publish,
            approvers: [alice],
            as,
        });
        await fixture.decide(alice, await asked(fixture, change, alice), "approve");

        return settled(fixture, change);
    };
    const elevated = await ask("approver");
    const reviewed = await ask("requester");

    // apply the elevated change as alice, and fail bob's own with the refusal of a note he cannot publish
    expect([elevated, reviewed, await title(fixture)]).toEqual([
        ["applied", null],
        ["failed", "PERMISSION_DENIED"],
        "approver",
    ]);
});

test("refuse alice's agent a call her grant gives once approved, and make it through the agent once she approves", async () => {
    // let alice's agent read her notes at once, publish them once approved, and open change requests for her
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const agent = agentOf(scope);
    const grantId = await fixture.grant(alice, agent, [
        { ...note.permission("read"), scope },
        { ...note.permission("publish"), scope, mode: "approval" as const },
        { ...changeRequest.permission("open"), scope },
    ]);
    const acting = AccessContext.of(
        { ...Caller.of(alice), actors: [agent], grants: [{ id: grantId, scope: alice.id }] },
        fixture.served.store.clock.now(),
    );

    // publish as the agent, refused for approval, then ask alice in a change request
    const publish = note.call("publish", { scope, id, title: "Final" });
    const refused = await refusal(
        fixture.served.store.mutate(acting, {
            id: RequestId.create(fixture.environment),
            calls: [publish],
        }),
    );
    const [opened] = await fixture.served.store.mutate(acting, {
        id: RequestId.create(fixture.environment),
        calls: [changeRequest.call("create", { scope, call: publish })],
    });
    const change = present(opened, "the change").id;
    await fixture.decide(alice, await asked(fixture, change, alice), "approve");

    // publish the note through her agent once she approved
    expect([refused, await settled(fixture, change), await title(fixture)]).toEqual([
        [
            "FORBIDDEN",
            "APPROVAL_REQUIRED",
            `publish on note ${id} needs the approval of the grant's delegator`,
        ],
        ["applied", null],
        "Final",
    ]);
});

test("count only approvals from others than the requester and its actors toward a role's approvals", async () => {
    // let bob publish alice's notes through a role once two others approved, and let his agent ask him
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const access = fixture.served.as(alice).client(objectPackage);
    const reviewed = await access.role.create({
        scope,
        requestId: RequestId.create(fixture.environment),
        name: "reviewed",
        description: "Publishes notes once two others approve",
        permissions: [{ ...note.permission("publish"), approvals: 2 }],
    });
    await fixture.bind(id, reviewed.id, bob);
    const publisher = await access.role.create({
        scope,
        requestId: RequestId.create(fixture.environment),
        name: "publisher",
        description: "Publishes notes",
        permissions: [note.permission("publish")],
    });
    await fixture.assign(publisher.id, carol);

    // publish as bob, refused with the approvals it needs
    const publish = note.call("publish", { scope, id, title: "Final" });
    const refused = await refusal(
        fixture.served
            .as(bob)
            .client(notesPackage)
            .note.publish({
                scope,
                requestId: RequestId.create(fixture.environment),
                id,
                title: "Final",
            }),
    );

    // ask alice once, add and approve bob's own approval: the change runs with one approval of others
    const padded = await openChange(fixture, bob, { id, call: publish, approvers: [alice] });
    await fixture.served
        .as(bob)
        .client(objectPackage)
        .approval.create({
            scope,
            requestId: RequestId.create(fixture.environment),
            parent: { packageId: changeRequest.package.id, type: changeRequest.name, id: padded },
            approverPackageId: bob.packageId,
            approverType: bob.type,
            approverScope: bob.scope,
            approverId: bob.id,
        });
    await fixture.decide(bob, await asked(fixture, padded, bob), "approve");
    await fixture.decide(alice, await asked(fixture, padded, alice), "approve");
    const first = await settled(fixture, padded);

    // ask alice and carol, who publish across the space, both approving: the change publishes the note as bob
    const second = await openChange(fixture, bob, {
        id,
        call: publish,
        approvers: [alice, carol],
        count: 2,
    });
    await fixture.decide(alice, await asked(fixture, second, alice), "approve");
    await fixture.decide(carol, await asked(fixture, second, carol), "approve");
    expect([refused, first, await settled(fixture, second), await title(fixture)]).toEqual([
        ["FORBIDDEN", "APPROVAL_REQUIRED", `publish on note ${id} needs 2 approvals`],
        ["failed", "APPROVAL_REQUIRED"],
        ["applied", null],
        "Final",
    ]);
});

test("fail an agent's change its delegator alone approves toward the delegator's own role approvals", async () => {
    // let bob publish alice's notes once one other approves, and his agent call for him, asking him by default
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const access = fixture.served.as(alice).client(objectPackage);
    const reviewed = await access.role.create({
        scope,
        requestId: RequestId.create(fixture.environment),
        name: "reviewed",
        description: "Publishes notes once another approves",
        permissions: [{ ...note.permission("publish"), approvals: 1 }],
    });
    await fixture.bind(id, reviewed.id, bob);
    const agent = agentOf(scope);
    const grantId = await fixture.grant(bob, agent, [
        { ...note.permission("read"), scope },
        { ...note.permission("publish"), scope },
        { ...note.permission("approve"), scope },
        { ...changeRequest.permission("open"), scope },
    ]);
    const acting = AccessContext.of(
        { ...Caller.of(bob), actors: [agent], grants: [{ id: grantId, scope: bob.id }] },
        fixture.served.store.clock.now(),
    );

    // open the agent's change, which asks bob, and approve it as bob
    const publish = note.call("publish", { scope, id, title: "Final" });
    const [opened] = await fixture.served.store.mutate(acting, {
        id: RequestId.create(fixture.environment),
        calls: [
            changeRequest.call("create", {
                scope,
                parent: { packageId: note.package.id, type: note.name, id },
                call: publish,
            }),
        ],
    });
    const change = present(opened, "the change").id;
    await fixture.decide(bob, await asked(fixture, change, bob), "approve");

    // fail the change: bob's own approval does not count toward his role's
    expect([await settled(fixture, change), await title(fixture)]).toEqual([
        ["failed", "APPROVAL_REQUIRED"],
        "Plan",
    ]);
});

test("refuse a requester asking to approve their own change, asking nobody, asking a group, or asking more approvals than its approvers give", async () => {
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const rename = note.call("update", { scope, id, title: "Final" });
    const owners = { ...principal.space.reference(Scope.universe.id, scope), relation: "owner" };
    expect(
        await Promise.all([
            refusal(openChange(fixture, alice, { id, call: rename, approvers: [alice] })),
            refusal(openChange(fixture, alice, { id, call: rename })),
            refusal(openChange(fixture, alice, { id, call: rename, approvers: [owners] })),
            refusal(openChange(fixture, alice, { id, call: rename, approvers: [bob], count: 2 })),
            refusal(
                openChange(fixture, alice, {
                    id,
                    call: rename,
                    approvers: [bob, carol],
                    as: "approver",
                }),
            ),
        ]),
    ).toEqual([
        ["BAD_REQUEST", "SELF_APPROVAL", "a requester cannot approve their own change"],
        ["BAD_REQUEST", "APPROVERS_MISSING", "a direct change request names no approvers"],
        [
            "BAD_REQUEST",
            "INPUT_INVALID",
            'invalid input: approvers.0: unrecognized key: "relation"',
        ],
        ["BAD_REQUEST", "QUORUM_UNREACHABLE", "2 approvals need at least 2 approvers, not 1"],
        [
            "BAD_REQUEST",
            "APPROVERS_INVALID",
            "a change run as its approver asks exactly one approver",
        ],
    ]);
});

test("refuse an agent deciding its delegator's approval, which the delegator then decides directly", async () => {
    // ask bob on alice's change, and let bob's agent read and approve notes for him
    const fixture = await openApprovals();
    const id = await fixture.write();
    const scope = fixture.served.scope;
    const rename = note.call("update", { scope, id, title: "Final" });
    const change = await openChange(fixture, alice, { id, call: rename, approvers: [bob] });
    const asking = await asked(fixture, change, bob);
    const agent = agentOf(scope);
    const grantId = await fixture.grant(bob, agent, [
        { ...note.permission("read"), scope },
        { ...note.permission("approve"), scope },
        { ...changeRequest.permission("approve"), scope },
        { ...approval.permission("decide"), scope },
    ]);
    const acting = AccessContext.of(
        { ...Caller.of(bob), actors: [agent], grants: [{ id: grantId, scope: bob.id }] },
        fixture.served.store.clock.now(),
    );

    // refuse the agent's decision, then apply the change once bob approves himself
    const refused = await refusal(
        fixture.served.store.mutate(acting, {
            id: RequestId.create(fixture.environment),
            calls: [approval.call("approve", approval.target(asking))],
        }),
    );
    await fixture.decide(bob, asking, "approve");
    expect([refused, await settled(fixture, change), await title(fixture)]).toEqual([
        ["FORBIDDEN", "DECIDER_REFUSED", "an approval is decided by its approver, acting directly"],
        ["applied", null],
        "Final",
    ]);
});
