import { defineScenario } from "@destack/package/declare";
import { codeInteraction } from "../scenario/code.ts";
import { Difference } from "./difference.ts";

/** Compare and merge files of about a megabyte, most of whose lines are alike. */
export const compareLargeFiles = defineScenario({
    of: Difference,
    interaction: codeInteraction,
    name: "compare-large-files",
    description:
        "compare a file of 27,000 lines with ones of which a hundredth, a twentieth and half changed, then merge two",
    given: { examples: [] },
    parameters: { times: 20 },
    when: [
        { step: { action: "write", lines: 27_000 } },
        {
            operation: "difference.few",
            times: "{times}",
            step: { action: "compare", share: 1 },
        },
        {
            operation: "difference.some",
            times: "{times}",
            step: { action: "compare", share: 5 },
        },
        {
            operation: "difference.many",
            times: "{times}",
            step: { action: "compare", share: 50 },
        },
        { operation: "difference.merge", times: "{times}", step: { action: "merge", share: 5 } },
    ],
    then: [{ observe: { count: "lines" }, is: 27_000 }],
});
