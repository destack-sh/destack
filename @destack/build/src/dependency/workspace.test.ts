import { expect, test } from "@destack/test";
import { Workspace } from "./workspace.ts";

test("meet a range with a host's builtin from its minimum on, unreleased code meeting every range and the dev tag only unreleased code", () => {
    const cases = [
        ["2026.11.0", ">=2026.11.0"],
        ["2026.12.3", ">=2026.11.0"],
        ["2026.10.0", ">=2026.11.0"],
        ["2026.11.0", "2026.11.0"],
        ["2026.11.1", "2026.11.0"],
        ["2026.11.0", "dev"],
        ["dev", ">=2026.11.0"],
        ["dev", "dev"],
        ["2026.11.0", "*"],
    ] as const;

    expect(cases.map(([version, range]) => Workspace.meets(version, range))).toEqual([
        true,
        true,
        false,
        true,
        false,
        false,
        true,
        true,
        true,
    ]);
});
