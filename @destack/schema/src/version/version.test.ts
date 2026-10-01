import { expect, test } from "@destack/test";
import { Version } from "./version.ts";

test("order calendar versions by year, month and release, nightly builds before their release", () => {
    const versions = [
        "2026.10.0",
        "2026.9.1",
        "2027.1.0",
        "2026.9.0",
        "2026.9.0-nightly.10",
        "2026.9.0-nightly.2",
        "2026.10.0-nightly.1",
    ];

    expect(versions.toSorted(Version.compare)).toEqual([
        "2026.9.0-nightly.2",
        "2026.9.0-nightly.10",
        "2026.9.0",
        "2026.9.1",
        "2026.10.0-nightly.1",
        "2026.10.0",
        "2027.1.0",
    ]);
});

test("accept only calendar versions", () => {
    const candidates = [
        "2026.9.0",
        "2026.12.3-nightly.0",
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
        ["2026.12.3-nightly.0", true],
        ["1.2.3", false],
        ["2026.13.0", false],
        ["2026.09.0", false],
        ["2026.9.0-beta.1", false],
        ["2026.9", false],
    ]);
});

test("list the releases after one release and up to another in release order", () => {
    const releases = ["2026.11.0", "2026.9.0", "2026.10.0", "2026.8.0", "2026.10.0-nightly.1"];

    expect([
        Version.between(releases, "2026.8.0", "2026.10.0"),
        Version.between(releases, "2026.10.0", "2026.10.0"),
        Version.between(releases, "2026.7.0", "2026.8.0"),
    ]).toEqual([["2026.9.0", "2026.10.0-nightly.1", "2026.10.0"], [], ["2026.8.0"]]);
});
