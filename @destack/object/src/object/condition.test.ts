import { expect, test } from "@destack/test";
import { AWAITING_APPROVAL, Status } from "./condition.ts";

/** A condition observed at a generation, its message naming its reason. */
function condition(status: "true" | "false", reason: string, observedGeneration = 2) {
    return { status, reason, message: `${reason} said`, observedGeneration, lastTransitionAt: 0 };
}

/** Read the progress, message and reason of a record at generation 2 with these conditions. */
function read(conditions: Readonly<Record<string, ReturnType<typeof condition>>>) {
    const record = { generation: 2, conditions };

    return [Status.progress(record), Status.message(record), Status.condition(record)?.reason];
}

test("read a record current once ready after a synced pass, failed on a lasting failure, waiting otherwise, with what it reports", () => {
    // read each state of a record whose latest change is generation 2
    const synced = condition("true", "ReconcileSuccess");
    expect({
        unobserved: read({}),
        earlierFailure: read({
            Ready: condition("true", "Deployed", 1),
            Synced: condition("false", "ReconcileError", 1),
        }),
        failed: read({
            Ready: condition("true", "Deployed", 1),
            Synced: condition("false", "ReconcileError"),
        }),
        approval: read({ Ready: condition("false", AWAITING_APPROVAL), Synced: synced }),
        waiting: read({ Ready: condition("false", "WaitingForResources"), Synced: synced }),
        staleReady: read({ Ready: condition("true", "Deployed", 1), Synced: synced }),
        current: read({ Ready: condition("true", "Deployed"), Synced: synced }),
    }).toEqual({
        unobserved: ["waiting", "", undefined],
        // wait on a failure of the earlier change, as no pass for the latest one ended yet
        earlierFailure: ["waiting", "Deployed said", "Deployed"],
        // fail with the pass's failure while the older ready condition still says the earlier change serves
        failed: ["failed", "ReconcileError said", "ReconcileError"],
        approval: ["approval", "AwaitingApproval said", "AwaitingApproval"],
        waiting: ["waiting", "WaitingForResources said", "WaitingForResources"],
        // wait until the pass reports readiness at the latest change
        staleReady: ["waiting", "Deployed said", "Deployed"],
        current: ["current", "Deployed said", "Deployed"],
    });
});
