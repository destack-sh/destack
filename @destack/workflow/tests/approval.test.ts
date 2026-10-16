import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { approval, change, changeRequest, defineWorkflow } from "../src/index.ts";
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

test("refuse a requester asking to approve their own change", async () => {
    const { id, open } = await openChange();
    expect(await refusal(open(id, [alice]))).toEqual([
        "BAD_REQUEST",
        "a requester cannot approve their own change",
    ]);
});
