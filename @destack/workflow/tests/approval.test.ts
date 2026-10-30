import { schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { principal } from "@destack/access";
import objectPackage from "@destack/object/package";
import { expect, single, test } from "@destack/test";
import { type Approval, approval, changeRequest } from "@destack/object";
import { defineWorkflow } from "../src/index.ts";
import {
    actor,
    alice,
    bob,
    note,
    notesPackage,
    openWorkflow,
    type TestWorkflow,
} from "./workflow.ts";
import { NOTES } from "./package.ts";
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
    readonly fixture: TestWorkflow;
    readonly id: string;
    readonly decide: (approver: "alice" | "bob", decision: "approve" | "reject") => Promise<void>;
}> {
    // review a note as its two approvers' run
    const fixture: TestWorkflow = await openWorkflow(review, async (step, input, context) =>
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

/** Decide an approval as its approver through the package's client. */
async function decideApproval(
    fixture: TestWorkflow,
    approver: typeof alice,
    asked: Approval,
    decision: "approve" | "reject",
): Promise<void> {
    const approvals = fixture.served.as(approver).client(objectPackage).approval;
    await approvals[decision]({
        ...approval.target(asked),
        requestId: RequestId.create(fixture.environment),
    });
}

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
    const fixture: TestWorkflow = await openWorkflow(release, async (step, input) => {
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
        { ...note.permission("read"), scope: spaceId },
        { ...note.permission("publish"), scope: spaceId, mode: "approval" as const },
        { ...changeRequest.permission("open"), scope: spaceId },
    ]);

    // start the run, and approve as alice once the change request asks her
    await fixture.start(() => release.start({ id }, actor()));
    const approvals = () => fixture.served.database.select().from(approval.table);
    await fixture.when(async () => (await approvals()).length > 0);
    const asked = single(await approvals());
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

test("fail a run whose call a role gives on two approvals, which a change asking only its delegator cannot gather", async () => {
    // publish a note in a run bob starts, bob's role giving publish once two others approve
    const fixture: TestWorkflow = await openWorkflow(release, async (step, input) => {
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
    const access = fixture.served.as(alice).client(objectPackage);
    const reviewed = await access.role.create({
        scope: spaceId,
        requestId: RequestId.create(fixture.environment),
        name: "reviewed",
        description: "Reads notes, publishes them once two others approve, and asks for approvals",
        permissions: [
            note.permission("read"),
            { ...note.permission("publish"), approvals: 2 },
            changeRequest.permission("open"),
        ],
    });
    await access.relationship.create({
        scope: spaceId,
        requestId: RequestId.create(fixture.environment),
        object: principal.space.reference(Scope.universe.id, spaceId),
        role: reviewed.id,
        subject: bob,
    });
    await fixture.grant(bob, fixture.served.store.as(release.run).subject, [
        { ...note.permission("read"), scope: spaceId },
        { ...note.permission("publish"), scope: spaceId },
        { ...changeRequest.permission("open"), scope: spaceId },
    ]);

    // start the run as bob, and expect it to fail once asking for the approvals is refused
    await fixture.start(() => release.start({ id }, actor(bob)));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "failed");
    expect([
        schema.looseObject({ reason: schema.string() }).parse(run?.["failure"]).reason,
    ]).toEqual(["APPROVERS_MISSING"]);
});
