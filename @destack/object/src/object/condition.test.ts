import { expect, test } from "@destack/test";
import { AWAITING_APPROVAL, Observation } from "./condition.ts";

/** A condition observed at a generation, with a status and a reason. */
function condition(status: "true" | "false", reason: string, observedGeneration = 2) {
    return { status, reason, message: "", observedGeneration, lastTransitionAt: 0 };
}

/** Read the progress of a record at generation 2 with these conditions. */
function progress(conditions: Readonly<Record<string, ReturnType<typeof condition>>>) {
    return Observation.progress({ generation: 2, conditions });
}

test("read a record current once ready after a synced pass, failed on a lasting failure, waiting otherwise", () => {
    // read each state of a record whose latest change is generation 2
    const synced = condition("true", "ReconcileSuccess");
    const unsynced = condition("false", "ReconcileError");
    expect({
        unobserved: progress({}),
        earlierFailure: progress({
            Ready: condition("true", "Deployed", 1),
            Synced: condition("false", "ReconcileError", 1),
        }),
        failed: progress({ Ready: condition("true", "Deployed", 1), Synced: unsynced }),
        approval: progress({ Ready: condition("false", AWAITING_APPROVAL), Synced: synced }),
        waiting: progress({ Ready: condition("false", "WaitingForResources"), Synced: synced }),
        staleReady: progress({ Ready: condition("true", "Deployed", 1), Synced: synced }),
        current: progress({ Ready: condition("true", "Deployed"), Synced: synced }),
    }).toEqual({
        unobserved: "waiting",
        // wait on a failure of the earlier change, as no pass for the latest one ended yet
        earlierFailure: "waiting",
        // fail while the older ready condition still says the earlier change serves
        failed: "failed",
        approval: "approval",
        waiting: "waiting",
        // wait until the pass reports readiness at the latest change
        staleReady: "waiting",
        current: "current",
    });
});
