import { expect, test } from "@destack/test";
import * as z from "zod";
import { describeIssues } from "./standard.ts";

test("describe issues as their paths and lowercase messages, a root issue without a path", () => {
    // validate a missing nested field and a wrong root
    const nested = z
        .object({ queries: z.object({ notes: z.string() }) })
        .safeParse({ queries: {} });
    const root = z.string().safeParse(1);

    expect(
        [nested.error?.issues, root.error?.issues].map((issues) => describeIssues(issues ?? [])),
    ).toEqual([
        "queries.notes: invalid input: expected string, received undefined",
        "invalid input: expected string, received number",
    ]);
});
