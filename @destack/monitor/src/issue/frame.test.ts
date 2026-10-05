import { expect, test } from "@destack/test";
import { parseStack } from "./frame.ts";

test("read V8, SpiderMonkey and JavaScriptCore frames, skipping messages and runtime frames and taking dependencies as not in app", () => {
    expect([
        parseStack(
            [
                "RangeError: page 9 of 3",
                "    at Note.rename (file:///srv/output/bun/workload.js:12:34)",
                "    at async run (file:///srv/output/bun/workload.js:40:5)",
                "    at file:///srv/output/bun/workload.js:2:1",
                "    at pad (file:///srv/node_modules/pad/index.js:1:1)",
                "    at processTicksAndRejections (node:internal/process/task_queues:105:5)",
            ].join("\n"),
        ),
        parseStack(
            "rename@https://destack.sh/assets/index-a1.js:1:2345\n@https://destack.sh/assets/index-a1.js:1:99",
        ),
    ]).toEqual([
        [
            {
                function: "Note.rename",
                file: "file:///srv/output/bun/workload.js",
                line: 12,
                column: 34,
                isInApp: true,
            },
            {
                function: "run",
                file: "file:///srv/output/bun/workload.js",
                line: 40,
                column: 5,
                isInApp: true,
            },
            { file: "file:///srv/output/bun/workload.js", line: 2, column: 1, isInApp: true },
            {
                function: "pad",
                file: "file:///srv/node_modules/pad/index.js",
                line: 1,
                column: 1,
                isInApp: false,
            },
        ],
        [
            {
                function: "rename",
                file: "https://destack.sh/assets/index-a1.js",
                line: 1,
                column: 2345,
                isInApp: true,
            },
            { file: "https://destack.sh/assets/index-a1.js", line: 1, column: 99, isInApp: true },
        ],
    ]);
});
