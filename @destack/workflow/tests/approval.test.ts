import workflowPackage from "../src/package.ts";
import { present, schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import { expect, refusal, test } from "@destack/test";
import { type Approval, approval, change, changeRequest, defineWorkflow } from "../src/index.ts";
import {
    actor,
    agentOf,
    alice,
    bob,
    note,
    notesPackage,
    openWorkflow,
    type WorkflowFixture,
} from "./fixture.ts";
import { NOTES } from "./package.ts";
import { NO_OUTPUT } from "../src/index.ts";
import { TEST_START } from "@destack/object/test";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** The input of a review: the note it asks about. */
const ReviewInput = schema.object({ id: schema.string() });

/** Ask bob and alice to approve a note, both approvals needed, waiting an hour at most. */
const review = defineWorkflow(
    {
        name: "review",
        input: ReviewInput,
        output: schema.enum(["requested", "approved", "rejected"]),
        waits: [approval],
    },
    NOTES,
);

/** Serve the review workflow and write the note it asks about. */
async function openReview(): Promise<{
    readonly fixture: WorkflowFixture;
    readonly id: string;
    readonly decide: (approver: "alice" | "bob", decision: "approve" | "reject") => Promise<void>;
}> {
    // review a note as its two approvers' run
    const fixture: WorkflowFixture = await openWorkflow(review, async (step, input, context) =>
        step.approve(
            "review",
            { packageId: note.package.id, type: note.name, scope: context.scope, id: input.id },
            { approvers: [bob, alice], count: 2, timeout: { hours: 1 } },
        ),
    );
    const { id } = await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "Plan",
        });

    // decide as an approver on their approval of the note
    const decide = async (approver: "alice" | "bob", decision: "approve" | "reject") => {
        const rows = await fixture.served.database.select().from(approval.table);
        const subject = approver === "alice" ? alice : bob;
        const mine = rows.find((row) => ObjectReference.at("approver", row).id === subject.id);
        if (mine === undefined) {
            throw new TypeError(`${approver} was not asked`);
        }
        await decideApproval(fixture, subject, mine, decision);
    };

    return { fixture, id, decide };
}

test("approve a note once both approvers approve, waiting while one has", async () => {
    // start a review, and let bob then alice approve
    const { fixture, id, decide } = await openReview();
    await fixture.start(() => review.start({ id }, actor()));
    await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    await decide("bob", "approve");
    const waiting = await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    await decide("alice", "approve");
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect([waiting[0]?.["state"], run?.["output"]]).toEqual(["waiting", "approved"]);
});

test("reject a note once one approver rejects, and ask both again in a later review", async () => {
    // reject the first review as bob
    const { fixture, id, decide } = await openReview();
    await fixture.start(() => review.start({ id }, actor()));
    await fixture.until((rows) => rows[0]?.["state"] === "waiting");
    await decide("alice", "approve");
    await decide("bob", "reject");
    const [first] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // start a second review, which asks both approvers again
    await fixture.start(() => review.start({ id }, actor()));
    await fixture.until(
        (rows) => rows.length === 2 && rows.some((row) => row["state"] === "waiting"),
    );
    const states = (await fixture.served.database.select().from(approval.table)).map(
        (row) => row.state,
    );
    expect([first?.["output"], states]).toEqual(["rejected", ["requested", "requested"]]);
});

test("answer a review nobody decides as requested once its hour passed", async () => {
    // start a review and let its hour pass
    const { fixture, id } = await openReview();
    await fixture.start(() => review.start({ id }, actor()));
    await fixture.simulation.jump(TEST_START + 60 * MINUTE);
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect(run?.["output"]).toBe("requested");
});

/** Do nothing, so a fixture serves the change requests alone. */
const idle = defineWorkflow({ name: "idle", input: schema.object({}), output: NO_OUTPUT }, NOTES);

/** Serve the change requests and write a note alice asks to rename. */
async function openChange(): Promise<{
    readonly fixture: WorkflowFixture;
    readonly id: string;
    readonly open: (
        target: string,
        approvers?: readonly Subject[],
        count?: number,
    ) => Promise<string>;
    readonly read: () => Promise<{ readonly state: string; readonly title: string }>;
}> {
    // write a note
    const fixture = await openWorkflow(idle, async () => {});
    const client = fixture.served.as(alice).client(notesPackage);
    const { id } = await client.note.create({
        spaceId: fixture.served.scope,
        requestId: RequestId.create(fixture.environment),
        title: "Plan",
    });
    const parent = { packageId: note.package.id, type: note.name, id };

    // open a change renaming a note, asking bob by default
    const open = async (target: string, approvers: readonly Subject[] = [bob], count = 1) => {
        const rename = note.call("update", {
            spaceId: fixture.served.scope,
            id: target,
            title: "Final",
        });
        const opened = await fixture.served
            .as(alice)
            .client(workflowPackage)
            .changeRequest.create({
                spaceId: fixture.served.scope,
                requestId: RequestId.create(fixture.environment),
                parent,
                call: rename,
                approvers: [...approvers],
                count,
            });

        return opened.id;
    };

    // read the change request's state and the note's title
    const read = async () => {
        const [requested] = await fixture.served.database.select().from(changeRequest.table);
        const [renamed] = await fixture.served.database.select().from(note.table);

        return { state: requested?.state ?? "absent", title: renamed?.title ?? "absent" };
    };

    return { fixture, id, open, read };
}

/** Decide an approval as its approver through the package's client. */
async function decideApproval(
    fixture: WorkflowFixture,
    approver: typeof alice,
    asked: Approval,
    decision: "approve" | "reject",
): Promise<void> {
    const approvals = fixture.served.as(approver).client(workflowPackage).approval;
    await approvals[decision]({
        ...approval.target(asked),
        requestId: RequestId.create(fixture.environment),
    });
}

/** Run the world until the change request reaches a state, as its run advances. */
async function settle(fixture: WorkflowFixture, state: string): Promise<void> {
    await fixture.when(async () => {
        const [requested] = await fixture.served.database.select().from(changeRequest.table);

        return requested?.state === state;
    });
}

test("rename a note as its requester once bob approves the change", async () => {
    // open the change and approve it as bob
    const { fixture, id, open, read } = await openChange();
    await open(id);
    await fixture.until(() => true);
    const [asked] = await fixture.served.database.select().from(approval.table);
    if (asked === undefined) {
        throw new TypeError("bob was not asked");
    }
    const before = await read();
    await decideApproval(fixture, bob, asked, "approve");
    await settle(fixture, "applied");
    expect([before, await read()]).toEqual([
        { state: "open", title: "Plan" },
        { state: "applied", title: "Final" },
    ]);
});

test("record the refusal an approved change's call answers", async () => {
    // approve a change renaming a missing note
    const { fixture, open } = await openChange();
    await open("note-01996ab0-0000-7000-8000-0000000000ff");
    await fixture.until(() => true);
    const [asked] = await fixture.served.database.select().from(approval.table);
    if (asked === undefined) {
        throw new TypeError("bob was not asked");
    }
    await decideApproval(fixture, bob, asked, "approve");
    await settle(fixture, "failed");
    const [failed] = await fixture.served.database.select().from(changeRequest.table);

    // expect the change failed with the update's refusal
    expect([failed?.state, failed?.failure?.code]).toEqual(["failed", "NOT_FOUND"]);
});

test("cancel the run of a change its requester withdraws", async () => {
    // open a change and withdraw it before bob decides
    const { fixture, id, open, read } = await openChange();
    const opened = await open(id);
    await fixture.until(() => true);
    await fixture.served
        .as(alice)
        .client(workflowPackage)
        .changeRequest.withdraw({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            id: opened,
        });
    await fixture.until(() => true);
    const runs = await fixture.served.database.select().from(change.run.table);
    expect([await read(), runs.map((row) => row["state"])]).toEqual([
        { state: "withdrawn", title: "Plan" },
        ["cancelled"],
    ]);
});

test("publish alice's note as alice once she approves bob's elevated change, and fail the same change as bob", async () => {
    // serve the change requests over alice's note, which bob, a member of her space, cannot write
    const fixture = await openWorkflow(idle, async () => {});
    const { id } = await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({
            spaceId: fixture.served.scope,
            requestId: RequestId.create(fixture.environment),
            title: "Plan",
        });
    const parent = { packageId: note.package.id, type: note.name, id };

    // look once the controllers settled
    const whenSettled = async <Value>(look: () => Promise<Value | undefined>): Promise<Value> => {
        await fixture.until(() => true);

        return present(await look(), "the change's settled state");
    };

    // ask alice to publish it, approve as alice, and read how the change settled
    const ask = async (as: "requester" | "approver") => {
        const rename = note.call("publish", { spaceId: fixture.served.scope, id, title: as });
        const opened = await fixture.served
            .as(bob)
            .client(workflowPackage)
            .changeRequest.create({
                spaceId: fixture.served.scope,
                requestId: RequestId.create(fixture.environment),
                parent,
                call: rename,
                approvers: [alice],
                as,
            });
        const asked = await whenSettled(async () =>
            (await fixture.served.database.select().from(approval.table)).find(
                (row) => row.state === "requested",
            ),
        );
        await decideApproval(fixture, alice, asked, "approve");

        return whenSettled(async () => {
            const rows = await fixture.served.database.select().from(changeRequest.table);
            const row = rows.find((each) => each.id === opened.id && each.state !== "open");

            return row === undefined ? undefined : [row.state, row.failure?.code ?? null];
        });
    };
    const elevated = await ask("approver");
    const reviewed = await ask("requester");
    const [renamed] = await fixture.served.database.select().from(note.table);

    // apply the elevated change as alice, and fail bob's own with the refusal of a note he cannot read
    expect([elevated, reviewed, renamed?.title]).toEqual([
        ["applied", null],
        ["failed", "NOT_FOUND"],
        "approver",
    ]);
});

test("refuse alice's agent a call her grant gives once approved, ask her in a change request, and make it through the agent once she approves", async () => {
    // let alice's agent read her notes at once, publish them once each use is approved, and open change requests for her
    const fixture = await openWorkflow(idle, async () => {});
    const spaceId = fixture.served.scope;
    const { id } = await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({ spaceId, requestId: RequestId.create(fixture.environment), title: "Plan" });
    const agent = agentOf(spaceId);
    const permissions = [
        { ...note.permission("read"), scope: spaceId },
        { ...note.permission("publish"), scope: spaceId, mode: "approval" as const },
        { ...changeRequest.permission("open"), scope: spaceId },
    ];
    const grantId = await fixture.grant(alice, agent, permissions);
    const caller = AccessContext.of(
        {
            ...Caller.of(alice),
            actors: [agent],
            grants: [{ id: grantId, scope: alice.id }],
        },
        fixture.simulation.clock.now(),
    );

    // publish as the agent, refused for approval, then ask alice in a change request
    const publish = note.call("publish", { spaceId, id, title: "Final" });
    const refused = await refusal(
        fixture.served.store.mutate(caller, {
            id: RequestId.create(fixture.environment),
            calls: [publish],
        }),
    );
    const [requested] = await fixture.served.store.mutate(caller, {
        id: RequestId.create(fixture.environment),
        calls: [changeRequest.call("create", { spaceId, call: publish })],
    });

    // approve as alice once asked, and publish the note through her agent
    await fixture.until(() => true);
    const [listed] = await fixture.served.database.select().from(approval.table);
    const asked: Approval = present(listed, "alice's approval");
    await decideApproval(fixture, alice, asked, "approve");
    await settle(fixture, "applied");
    const [published] = await fixture.served.database.select().from(note.table);
    const [request] = await fixture.served.database.select().from(changeRequest.table);
    expect([refused, request?.id, ObjectReference.at("approver", asked), published?.title]).toEqual(
        [
            [
                "FORBIDDEN",
                "APPROVAL_REQUIRED",
                `grant ${grantId} gives publish on note ${id} only after approval`,
            ],
            present(requested, "the change request").id,
            alice,
            "Final",
        ],
    );
});

/** Publish a note in a run, answering its published title. */
const release = defineWorkflow(
    {
        name: "release",
        input: schema.object({ id: schema.string() }),
        output: schema.object({ title: schema.string() }),
    },
    NOTES,
);

test("wait at a call alice's grant gives once approved, ask her in a change request, and answer the call once she approves", async () => {
    // publish a note in a run alice starts, her grant to the serving installation giving publish once approved
    const fixture: WorkflowFixture = await openWorkflow(release, async (step, input) => {
        const publish = note.call("publish", {
            spaceId: fixture.served.scope,
            id: input.id,
            title: "Final",
        });
        const published = await step.call("publish", publish);

        return { title: published.title };
    });
    const spaceId = fixture.served.scope;
    const { id } = await fixture.served
        .as(alice)
        .client(notesPackage)
        .note.create({ spaceId, requestId: RequestId.create(fixture.environment), title: "Plan" });
    await fixture.grant(alice, fixture.served.store.as(release.run).subject, [
        { ...note.permission("publish"), scope: spaceId, mode: "approval" as const },
        { ...changeRequest.permission("open"), scope: spaceId },
    ]);

    // start the run, and approve as alice once the change request asks her
    await fixture.start(() => release.start({ id }, actor()));
    let asked: Approval | undefined;
    for (let look = 0; look < 1000 && asked === undefined; look++) {
        await fixture.until(() => true);
        [asked] = await fixture.served.database.select().from(approval.table);
    }
    if (asked === undefined) {
        throw new TypeError("alice was not asked");
    }
    await decideApproval(fixture, alice, asked, "approve");
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect alice asked, the change applied, and the run answering the published title
    const [request] = await fixture.served.database.select().from(changeRequest.table);
    expect([ObjectReference.at("approver", asked), request?.state, run?.["output"]]).toEqual([
        alice,
        "applied",
        { title: "Final" },
    ]);
});

test("refuse a requester asking to approve their own change, asking nobody, asking a group, or asking more approvals than its approvers give", async () => {
    const { fixture, id, open } = await openChange();
    const spaceId = fixture.served.scope;
    const unasked = fixture.served
        .as(alice)
        .client(workflowPackage)
        .changeRequest.create({
            spaceId,
            requestId: RequestId.create(fixture.environment),
            parent: { packageId: note.package.id, type: note.name, id },
            call: note.call("update", { spaceId, id, title: "Final" }),
        });
    const owners = { ...principal.space.reference(Scope.universe.id, spaceId), relation: "owner" };
    expect(
        await Promise.all([
            refusal(open(id, [alice])),
            refusal(unasked),
            refusal(open(id, [owners])),
            refusal(open(id, [bob], 2)),
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
    ]);
});
