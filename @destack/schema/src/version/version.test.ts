import { expect, test } from "@destack/test";
import { Version } from "./version.ts";

test("order releases by year, month and release, and dev after every release", () => {
    const versions = ["2026.10.0", "dev", "2026.9.1", "2027.1.0", "2026.9.0"];

    expect(versions.toSorted((left, right) => Version.compare(left, right))).toEqual([
        "2026.9.0",
        "2026.9.1",
        "2026.10.0",
        "2027.1.0",
        "dev",
    ]);
});

test("accept calendar releases and dev", () => {
    const candidates = [
        "2026.9.0",
        "dev",
        "2026.12.3-dev",
        "1.2.3",
        "2026.13.0",
        "2026.09.0",
        "2026.9.0-beta.1",
        "2026.9",
    ];

    expect(
        candidates.map((candidate) => [candidate, Version.safeParse(candidate).success]),
    ).toEqual([
        ["2026.9.0", true],
        ["dev", true],
        ["2026.12.3-dev", false],
        ["1.2.3", false],
        ["2026.13.0", false],
        ["2026.09.0", false],
        ["2026.9.0-beta.1", false],
        ["2026.9", false],
    ]);
});

test("write the first release of a time's UTC month", () => {
    expect([
        Version.first(Date.UTC(2026, 10, 17, 3)),
        Version.first(Date.UTC(2026, 11, 31, 23, 59)),
        Version.first(Date.UTC(2027, 0, 1)),
    ]).toEqual(["2026.11.0", "2026.12.0", "2027.1.0"]);
});

test("name the next release in a release's year and month, and refuse dev", () => {
    expect([
        ["2026.9.0", "2026.12.4"].map((version) => Version.next(version)),
        [Version.isRelease("2026.9.0"), Version.isRelease(Version.development)],
    ]).toEqual([
        ["2026.9.1", "2026.12.5"],
        [true, false],
    ]);
    expect(() => Version.next(Version.development)).toThrow(
        new TypeError("version 'dev' is not a release"),
    );
});

test("meet a range from its minimum on, unreleased code meeting every range and the dev tag only unreleased code", () => {
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

    expect(cases.map(([version, range]) => Version.meets(version, range))).toEqual([
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
