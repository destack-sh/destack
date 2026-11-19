import { expect, test } from "vitest";
import { TestPlan } from "./plan.ts";

test("derive one target per tier, environment and database, scenario modules benched, types once", () => {
    // plan a package's files of every kind against both databases
    const targets = TestPlan.derive(
        [
            "src/note/note.test.ts",
            "src/note/note.test.tsx",
            "src/note/note.server.test.ts",
            "src/note/note.hydration.test.tsx",
            "src/note/note.scenario.ts",
            "tests/sync.scenario.test.ts",
            "tests/types.test-d.ts",
            "tests/lint.node.test.ts",
            "src/note/note.ts",
        ],
        ["sqlite", "postgresql"],
        "src/test/setup.ts",
    );

    // expect each kind in its target per database, the module benched and types without one
    expect(targets.map((each) => [each.name, each.files])).toEqual([
        ["unit:hydration:sqlite", ["src/note/note.hydration.test.tsx"]],
        ["unit:hydration:postgresql", ["src/note/note.hydration.test.tsx"]],
        ["scenario:bun:sqlite", ["src/note/note.scenario.ts", "tests/sync.scenario.test.ts"]],
        ["scenario:bun:postgresql", ["src/note/note.scenario.ts", "tests/sync.scenario.test.ts"]],
        ["bench:bun:sqlite", ["src/note/note.scenario.ts"]],
        ["bench:bun:postgresql", ["src/note/note.scenario.ts"]],
        ["unit:server:sqlite", ["src/note/note.server.test.ts"]],
        ["unit:server:postgresql", ["src/note/note.server.test.ts"]],
        ["unit:bun:sqlite", ["src/note/note.test.ts"]],
        ["unit:bun:postgresql", ["src/note/note.test.ts"]],
        ["unit:dom:sqlite", ["src/note/note.test.tsx"]],
        ["unit:dom:postgresql", ["src/note/note.test.tsx"]],
        ["unit:node:sqlite", ["tests/lint.node.test.ts"]],
        ["unit:node:postgresql", ["tests/lint.node.test.ts"]],
        ["unit:typecheck", ["tests/types.test-d.ts"]],
    ]);
});
