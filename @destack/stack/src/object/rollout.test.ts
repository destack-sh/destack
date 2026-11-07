import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Rollout } from "./rollout.ts";

/** The revision the rollouts target. */
const TARGETED = schema
    .identifier("installation-revision")
    .parse("installation-revision-01996ab0-0000-7000-8000-0000000000a1");

/** The revision the space served before. */
const EARLIER = schema
    .identifier("installation-revision")
    .parse("installation-revision-01996ab0-0000-7000-8000-0000000000a2");

/** A condition observed at a generation, with a status and a reason. */
function condition(status: "true" | "false", reason: string, observedGeneration = 1) {
    return { status, reason, message: "", observedGeneration, lastTransitionAt: 0 };
}

/** Read the progress of a rollout serving a revision, from its readiness and synced conditions. */
function progress(
    served: typeof TARGETED | null,
    observed: {
        readonly ready?: ReturnType<typeof condition>;
        readonly synced?: ReturnType<typeof condition>;
        readonly installationGeneration?: number;
        readonly rolloutGeneration?: number;
    },
) {
    const { ready, synced, installationGeneration = 1, rolloutGeneration = 1 } = observed;

    return Rollout.progress({
        revisionId: TARGETED,
        servedRevisionId: served,
        conditions: synced === undefined ? {} : { Synced: synced },
        generation: rolloutGeneration,
        installation: {
            conditions: ready === undefined ? {} : { Ready: ready },
            generation: installationGeneration,
        },
    });
}

test("read a rollout served once its space serves the target, waiting until its installation's latest change is observed, and failed on a refusal or a lasting failure", () => {
    // read each state of a rollout targeting a revision
    const unready = condition("false", "WaitingForResources");
    const unsynced = condition("false", "ReconcileError");
    expect({
        served: progress(TARGETED, {}),
        unobserved: progress(EARLIER, {}),
        deploying: progress(EARLIER, { ready: condition("true", "Deployed") }),
        waiting: progress(EARLIER, { ready: unready }),
        refused: progress(EARLIER, { ready: condition("false", "UndeclaredCapability") }),
        unsynced: progress(EARLIER, { ready: unready, synced: unsynced }),
        revised: progress(EARLIER, { ready: unready, synced: unsynced, rolloutGeneration: 2 }),
        reinstalled: progress(EARLIER, {
            ready: condition("false", "UndeclaredCapability"),
            synced: unsynced,
            installationGeneration: 2,
        }),
    }).toEqual({
        served: "served",
        unobserved: "waiting",
        deploying: "waiting",
        waiting: "waiting",
        refused: "failed",
        unsynced: "failed",
        // wait on a failure an earlier rollout recorded, as it retries since
        revised: "waiting",
        // wait on a refusal and a failure of the installation's earlier generation, as its controller has not seen the change yet
        reinstalled: "waiting",
    });
});
