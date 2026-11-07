import { expect, test } from "@destack/test";
import { AWAITING_APPROVAL, Observation } from "./condition.ts";

/** A condition observed at a generation, its message naming its reason. */
function condition(status: "true" | "false", reason: string, observedGeneration = 2) {
    return { status, reason, message: `${reason} said`, observedGeneration, lastTransitionAt: 0 };
}

/** Read the progress and message of a record at generation 2 with these conditions. */
function read(conditions: Readonly<Record<string, ReturnType<typeof condition>>>) {
    const record = { generation: 2, conditions };

    return [Observation.progress(record), Observation.message(record)];
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
        unobserved: ["waiting", ""],
        // wait on a failure of the earlier change, as no pass for the latest one ended yet
        earlierFailure: ["waiting", "Deployed said"],
        // fail with the pass's failure while the older ready condition still says the earlier change serves
        failed: ["failed", "ReconcileError said"],
        approval: ["approval", "AwaitingApproval said"],
        waiting: ["waiting", "WaitingForResources said"],
        // wait until the pass reports readiness at the latest change
        staleReady: ["waiting", "Deployed said"],
        current: ["current", "Deployed said"],
    });
});
