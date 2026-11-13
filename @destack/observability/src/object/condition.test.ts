import { ids } from "@destack/machine/test";
import { expect, refusal, test } from "@destack/test";
import { type AlertCondition, AlertRule, log, metric, NO_PENDING } from "./index.ts";

/** A condition over error records, as a rule declares it. */
const ERRORS: AlertCondition = {
    kind: "events",
    event: log.typeReference,
    where: "severity >= 17",
    fold: "count",
    group: ["installationId"],
    window: { minutes: 5 },
    comparison: "above",
    threshold: 2,
};

/** Check a rule of a condition and pending period against the space's log and metric types. */
function check(condition: AlertCondition, pending = NO_PENDING) {
    return refusal(async () =>
        AlertRule.require({ scope: ids.space, condition, pending, actions: [{ kind: "notify" }] }, [
            log,
            metric,
        ]),
    );
}

test("refuse a pending issue condition, a quantile of a kind without distributions, and a sum without its measure", async () => {
    expect([
        await check({ kind: "issue", on: "open" }, { minutes: 1 }),
        await check({ ...ERRORS, fold: "p99" }),
        await check({ ...ERRORS, fold: "sum" }),
    ]).toEqual([
        ["BAD_REQUEST", "ISSUE_INVALID", "issue condition is invalid: it takes no pending time"],
        [
            "BAD_REQUEST",
            "DISTRIBUTION_REQUIRED",
            "kind distribution is required: a 'p99' fold can only read a kind's distributions",
        ],
        ["BAD_REQUEST", "MEASURE_REQUIRED", "measured key is required: a 'sum' fold reads it"],
    ]);
});
