import { expect, test } from "@destack/test";
import { Version } from "./version.ts";

test("order calendar versions by year, month and release, dev builds before their release", () => {
    const versions = [
        "2026.10.0",
        "2026.9.1",
        "2027.1.0",
        "2026.9.0",
        "2026.9.0-dev.20260912100000.0123456789ab",
        "2026.9.0-dev.20260912090000.0123456789ab",
        "2026.10.0-dev.20261001090000.abcdef012345",
    ];

    expect(versions.toSorted((left, right) => Version.compare(left, right))).toEqual([
        "2026.9.0-dev.20260912090000.0123456789ab",
        "2026.9.0-dev.20260912100000.0123456789ab",
        "2026.9.0",
        "2026.9.1",
        "2026.10.0-dev.20261001090000.abcdef012345",
        "2026.10.0",
        "2027.1.0",
    ]);
});

test("accept only calendar versions", () => {
    const candidates = [
        "2026.9.0",
        "2026.12.3-dev.20261201090000.00ff00ff00ff",
        "2026.12.3-dev.4",
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
        ["2026.12.3-dev.20261201090000.00ff00ff00ff", true],
        ["2026.12.3-dev.4", false],
        ["1.2.3", false],
        ["2026.13.0", false],
        ["2026.09.0", false],
        ["2026.9.0-beta.1", false],
        ["2026.9", false],
    ]);
});

test("list the releases after one version's release and up to another's in release order", () => {
    const releases = { "2026.11.0": 4, "2026.9.0": 2, "2026.10.0": 3, "2026.8.0": 1 };

    // a dev build leads to its release and carries its entries
    expect([
        Version.between(releases, "2026.8.0", "2026.10.0"),
        Version.between(releases, "2026.10.0", "2026.10.0"),
        Version.between(releases, "2026.7.0", "2026.8.0"),
        Version.between(releases, "2026.9.0", "2026.10.0-dev.20261001090000.abcdef012345"),
        Version.between(
            releases,
            "2026.10.0-dev.20261001090000.abcdef012345",
            "2026.10.0-dev.20261002090000.abcdef012345",
        ),
        Version.between(releases, "2026.10.0-dev.20261002090000.abcdef012345", "2026.10.0"),
    ]).toEqual([
        [
            ["2026.9.0", 2],
            ["2026.10.0", 3],
        ],
        [],
        [["2026.8.0", 1]],
        [["2026.10.0", 3]],
        [],
        [],
    ]);
});

test("refuse conversion keys that are dev builds or follow the declaring version's release", () => {
    // a dev build declares the conversions of the release it leads to
    expect([
        refusal({ "2026.9.0": 1 }, "2026.9.0-dev.20261001090000.abcdef012345"),
        refusal({ "2026.9.0-dev.20261001090000.abcdef012345": 1 }, "2026.9.0"),
        refusal({ "2026.10.0": 1 }, "2026.9.0-dev.20260904090000.0a0b0c0d0e0f"),
    ]).toEqual([
        "accepted",
        "TypeError: conversion key of 'user' is not a release: '2026.9.0-dev.20261001090000.abcdef012345'",
        "TypeError: conversion of 'user' is keyed by '2026.10.0', after its version '2026.9.0-dev.20260904090000.0a0b0c0d0e0f'",
    ]);
});

test("name the release after a version's release in its year and month", () => {
    expect(
        ["2026.9.0", "2026.12.4", "2026.9.0-dev.20260903090000.0a0b0c0d0e0f"].map((version) =>
            Version.next(version),
        ),
    ).toEqual(["2026.9.1", "2026.12.5", "2026.9.1"]);
});

test("write a dev build's version from its release, time and key", () => {
    expect(Version.dev("2026.9.0", Date.UTC(2026, 8, 12, 10, 30, 5), "0123456789abcdef")).toBe(
        "2026.9.0-dev.20260912103005.0123456789ab",
    );
});

test("tell dev builds from releases", () => {
    const versions = [
        "2026.9.0",
        "2026.9.0-dev.20260903090000.0a0b0c0d0e0f",
        "2026.10.1-dev.20261201090000.00ff00ff00ff",
    ];

    expect(versions.map((version) => Version.isPrerelease(version))).toEqual([false, true, true]);
});

/** Check a version's conversion keys, reading a refusal as its message. */
function refusal(keyed: Readonly<Record<string, number>>, version: string): string {
    // accept the keys, or read why they are refused
    try {
        Version.requireUpTo(keyed, version, "user");

        return "accepted";
    } catch (error) {
        return String(error);
    }
}
