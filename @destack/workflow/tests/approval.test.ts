import { present, schema } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { expect, refusal, test } from "@destack/test";
import { type Approval, approval, change, changeRequest, defineWorkflow } from "../src/index.ts";
import { actor, alice, bob, note, openWorkflow, type WorkflowFixture } from "./fixture.ts";

/** The input of a review: the note it asks about. */
const ReviewInput = schema.object({ id: schema.string() });

/** Ask bob and alice to approve a note, both approvals needed, waiting an hour at most. */
const review = defineWorkflow({
    name: "review",
    input: ReviewInput,
    output: schema.enum(["requested", "approved", "rejected"]),
    waits: [approval],
});

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
    const [written] = await fixture.asAlice([
        note.call("create", { spaceId: fixture.spaceId, title: "Plan" }),
    ]);
    const { id } = schema.looseObject({ id: schema.string() }).parse(written);

    // decide as an approver on their approval of the note
    const decide = async (approver: "alice" | "bob", decision: "approve" | "reject") => {
        const rows = await fixture.store.database.select().from(approval.table);
        const subject = approver === "alice" ? alice : bob;
        const mine = rows.find((row) => ObjectReference.at("approver", row).id === subject.id);
        if (mine === undefined) {
            throw new TypeError(`${approver} was not asked`);
        }
        const call = [approval.call(decision, approval.target(mine))];
        await (approver === "alice" ? fixture.asAlice(call) : fixture.asBob(call));
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
    const states = (await fixture.store.database.select().from(approval.table)).map(
        (row) => row.state,
    );
    expect([first?.["output"], states]).toEqual(["rejected", ["requested", "requested"]]);
});

test("answer a review nobody decides as requested once its hour passed", async () => {
    // start a review and let the hour pass in minute steps
    const { fixture, id } = await openReview();
    await fixture.start(() => review.start({ id }, actor()));
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");
    expect(run?.["output"]).toBe("requested");
});

/** Do nothing, so a fixture serves the change requests alone. */
const idle = defineWorkflow({ name: "idle", input: schema.object({}) });

/** Serve the change requests and write a note alice asks to rename. */
async function openChange(): Promise<{
    readonly fixture: WorkflowFixture;
    readonly id: string;
    readonly open: (target: string, approvers?: readonly (typeof bob)[]) => Promise<string>;
    readonly read: () => Promise<{ readonly state: string; readonly title: string }>;
}> {
    // write a note
    const fixture = await openWorkflow(idle, async () => {});
    const [written] = await fixture.asAlice([
        note.call("create", { spaceId: fixture.spaceId, title: "Plan" }),
    ]);
    const { id } = schema.looseObject({ id: schema.string() }).parse(written);
    const parent = { packageId: note.package.id, type: note.name, id };

    // open a change renaming a note, asking bob by default
    const open = async (target: string, approvers: readonly (typeof bob)[] = [bob]) => {
        const rename = note.call("update", {
            spaceId: fixture.spaceId,
            id: target,
            title: "Final",
        });
        const [opened] = await fixture.asAlice([
            changeRequest.call("create", {
                spaceId: fixture.spaceId,
                parent,
                call: rename,
                approvers: [...approvers],
            }),
        ]);

        return schema.looseObject({ id: schema.string() }).parse(opened).id;
    };

    // read the change request's state and the note's title
    const read = async () => {
        const [requested] = await fixture.store.database.select().from(changeRequest.table);
        const [renamed] = await fixture.store.database.select().from(note.table);

        return { state: requested?.state ?? "absent", title: renamed?.title ?? "absent" };
    };

    return { fixture, id, open, read };
}

/** Wait until the change requests settle, as the change runs advance. */
async function settle(fixture: WorkflowFixture, state: string): Promise<void> {
    for (let look = 0; look < 1000; look++) {
        await fixture.until(() => true);
        const [requested] = await fixture.store.database.select().from(changeRequest.table);
        if (requested?.state === state) {
            return;
        }
    }
    throw new Error(`the change request did not become ${state}`);
}

test("rename a note as its requester once bob approves the change", async () => {
    // open the change and approve it as bob
    const { fixture, id, open, read } = await openChange();
    await open(id);
    await fixture.until(() => true);
    const [asked] = await fixture.store.database.select().from(approval.table);
    if (asked === undefined) {
        throw new TypeError("bob was not asked");
    }
    const before = await read();
    await fixture.asBob([approval.call("approve", approval.target(asked))]);
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
    const [asked] = await fixture.store.database.select().from(approval.table);
    if (asked === undefined) {
        throw new TypeError("bob was not asked");
    }
    await fixture.asBob([approval.call("approve", approval.target(asked))]);
    await settle(fixture, "failed");
    const [failed] = await fixture.store.database.select().from(changeRequest.table);

    // expect the change failed with the update's refusal
    expect([failed?.state, failed?.failure?.code]).toEqual(["failed", "NOT_FOUND"]);
});

test("cancel the run of a change its requester withdraws", async () => {
    // open a change and withdraw it before bob decides
    const { fixture, id, open, read } = await openChange();
    const opened = await open(id);
    await fixture.until(() => true);
    await fixture.asAlice([
        changeRequest.call(
            "withdraw",
            changeRequest.target({ scope: fixture.spaceId, id: opened }),
        ),
    ]);
    await fixture.until(() => true);
    const runs = await fixture.store.database.select().from(change.run.table);
    expect([await read(), runs.map((row) => row["state"])]).toEqual([
        { state: "withdrawn", title: "Plan" },
        ["cancelled"],
    ]);
});

test("publish bob's note as bob once he approves alice's elevated change, and fail the same change as alice", async () => {
    // serve the change requests over bob's note, which alice cannot write
    const fixture = await openWorkflow(idle, async () => {});
    const [written] = await fixture.asBob([
        note.call("create", { spaceId: fixture.spaceId, title: "Plan" }),
    ]);
    const { id } = schema.looseObject({ id: schema.string() }).parse(written);
    const parent = { packageId: note.package.id, type: note.name, id };

    // look once the controllers settled
    const whenSettled = async <Value>(look: () => Promise<Value | undefined>): Promise<Value> => {
        await fixture.until(() => true);

        return present(await look(), "the change's settled state");
    };

    // ask bob to publish it, approve as bob, and read how the change settled
    const ask = async (as: "requester" | "approver") => {
        const rename = note.call("publish", { spaceId: fixture.spaceId, id, title: as });
        const [opened] = await fixture.asAlice([
            changeRequest.call("create", {
                spaceId: fixture.spaceId,
                parent,
                call: rename,
                approvers: [bob],
                as,
            }),
        ]);
        const { id: requestId } = schema.looseObject({ id: schema.string() }).parse(opened);
        const asked = await whenSettled(async () =>
            (await fixture.store.database.select().from(approval.table)).find(
                (row) => row.state === "requested",
            ),
        );
        await fixture.asBob([approval.call("approve", approval.target(asked))]);

        return whenSettled(async () => {
            const rows = await fixture.store.database.select().from(changeRequest.table);
            const row = rows.find((each) => each.id === requestId && each.state !== "open");

            return row === undefined ? undefined : [row.state, row.failure?.code ?? null];
        });
    };
    const elevated = await ask("approver");
    const reviewed = await ask("requester");
    const [renamed] = await fixture.store.database.select().from(note.table);

    // apply the elevated change as bob, and fail alice's own with the publish refusal
    expect([elevated, reviewed, renamed?.title]).toEqual([
        ["applied", null],
        ["failed", "NOT_FOUND"],
        "approver",
    ]);
});

test("refuse alice's agent a call her grant gives once approved, ask her in a change request, and make it through the agent once she approves", async () => {
    // let alice's agent read her notes at once, publish them once each use is approved, and open change requests for her
    const fixture = await openWorkflow(idle, async () => {});
    const [written] = await fixture.asAlice([
        note.call("create", { spaceId: fixture.spaceId, title: "Plan" }),
    ]);
    const { id } = schema.looseObject({ id: schema.string() }).parse(written);
    const agent = principal.installation.reference(fixture.spaceId, "installation-agent");
    const permissions = [
        { ...note.permission("read"), scope: fixture.spaceId },
        { ...note.permission("publish"), scope: fixture.spaceId, mode: "approval" as const },
        { ...changeRequest.permission("open"), scope: fixture.spaceId },
    ];
    const grantId = await new AccessFixture(fixture.store.database).replicateGrant(
        alice,
        agent,
        permissions,
    );
    const caller = AccessContext.of(
        {
            ...Caller.of(alice),
            actors: [agent],
            grants: [{ id: grantId, scope: alice.id }],
        },
        fixture.clock.now(),
    );

    // publish as the agent, refused for approval, then ask alice in a change request
    const publish = note.call("publish", { spaceId: fixture.spaceId, id, title: "Final" });
    const refused = await refusal(
        fixture.store.mutate(caller, {
            id: RequestId.create(fixture.clock.now()),
            calls: [publish],
        }),
    );
    const [requested] = await fixture.store.mutate(caller, {
        id: RequestId.create(fixture.clock.now()),
        calls: [changeRequest.call("create", { spaceId: fixture.spaceId, call: publish })],
    });

    // approve as alice once asked, and publish the note through her agent
    await fixture.until(() => true);
    const [listed] = await fixture.store.database.select().from(approval.table);
    const asked: Approval = present(listed, "alice's approval");
    await fixture.asAlice([approval.call("approve", approval.target(asked))]);
    await settle(fixture, "applied");
    const [published] = await fixture.store.database.select().from(note.table);
    const [request] = await fixture.store.database.select().from(changeRequest.table);
    expect([refused, request?.id, ObjectReference.at("approver", asked), published?.title]).toEqual(
        [
            ["APPROVAL_REQUIRED", `grant ${grantId} gives publish on note ${id} once approved`],
            present(requested, "the change request").id,
            alice,
            "Final",
        ],
    );
});

/** Publish a note in a run, answering its published title. */
const release = defineWorkflow({
    name: "release",
    input: schema.object({ id: schema.string() }),
    output: schema.object({ title: schema.string() }),
});

test("wait at a call alice's grant gives once approved, ask her in a change request, and answer the call once she approves", async () => {
    // publish a note in a run alice starts, her grant to the serving installation giving publish once approved
    const fixture: WorkflowFixture = await openWorkflow(release, async (step, input) => {
        const publish = note.call("publish", {
            spaceId: fixture.spaceId,
            id: input.id,
            title: "Final",
        });
        const published = await step.call("publish", publish);

        return { title: published.title };
    });
    const [written] = await fixture.asAlice([
        note.call("create", { spaceId: fixture.spaceId, title: "Plan" }),
    ]);
    const { id } = schema.looseObject({ id: schema.string() }).parse(written);
    await new AccessFixture(fixture.store.database).replicateGrant(
        alice,
        fixture.store.serving(release.run).subject,
        [
            { ...note.permission("publish"), scope: fixture.spaceId, mode: "approval" as const },
            { ...changeRequest.permission("open"), scope: fixture.spaceId },
        ],
        fixture.clock.now(),
    );

    // start the run, and approve as alice once the change request asks her
    await fixture.start(() => release.start({ id }, actor()));
    let asked: Approval | undefined;
    for (let look = 0; look < 1000 && asked === undefined; look++) {
        await fixture.until(() => true);
        [asked] = await fixture.store.database.select().from(approval.table);
    }
    if (asked === undefined) {
        throw new TypeError("alice was not asked");
    }
    await fixture.asAlice([approval.call("approve", approval.target(asked))]);
    const [run] = await fixture.until((rows) => rows[0]?.["state"] === "completed");

    // expect alice asked, the change applied, and the run answering the published title
    const [request] = await fixture.store.database.select().from(changeRequest.table);
    expect([ObjectReference.at("approver", asked), request?.state, run?.["output"]]).toEqual([
        alice,
        "applied",
        { title: "Final" },
    ]);
});

test("refuse a requester asking to approve their own change", async () => {
    const { id, open } = await openChange();
    expect(await refusal(open(id, [alice]))).toEqual([
        "BAD_REQUEST",
        "a requester cannot approve their own change",
    ]);
});
