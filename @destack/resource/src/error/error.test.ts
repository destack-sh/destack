import { expect, test } from "@destack/test";
import { ResourceError, unreachablePlan } from "./error.ts";

test("refuse a plan no declaration change reaches as a conflict listing every problem", () => {
    const problems = [
        { target: "table/note", detail: "declare a conversion to version 2" },
        { target: "table/tag", detail: "declare the dropped column" },
    ];
    const failure = unreachablePlan(problems);

    // list every problem in the message and the data
    expect([failure.toCallError(), ResourceError.is(failure, "PLAN_UNREACHABLE")]).toEqual([
        {
            code: "CONFLICT",
            reason: "PLAN_UNREACHABLE",
            domain: "@destack/resource",
            message:
                "table/note: declare a conversion to version 2; table/tag: declare the dropped column",
            data: { problems },
        },
        true,
    ]);
});
