import { expect, test } from "@destack/test";
import { Version } from "./version.ts";

test("order calendar versions by year, month and release, dev builds before their release", () => {
    const versions = [
        "2026.10.0",
        "2026.9.1",
        "2027.1.0",
        "2026.9.0",
        "2026.9.0-dev",
        "2026.10.0-dev",
    ];

    expect(versions.toSorted((left, right) => Version.compare(left, right))).toEqual([
        "2026.9.0-dev",
        "2026.9.0",
        "2026.9.1",
        "2026.10.0-dev",
        "2026.10.0",
        "2027.1.0",
    ]);
});

test("accept only calendar versions", () => {
    const candidates = [
        "2026.9.0",
        "2026.12.3-dev",
        "2026.12.3-dev.20261201090000.00ff00ff00ff",
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
        ["2026.12.3-dev", true],
        ["2026.12.3-dev.20261201090000.00ff00ff00ff", false],
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

test("name the release after a version's release in its year and month", () => {
    expect(
        ["2026.9.0", "2026.12.4", "2026.9.0-dev"].map((version) => Version.next(version)),
    ).toEqual(["2026.9.1", "2026.12.5", "2026.9.1"]);
});

test("write a dev build's version from its release, the same for every build", () => {
    expect([Version.dev("2026.9.0"), Version.dev("2026.9.0-dev")]).toEqual([
        "2026.9.0-dev",
        "2026.9.0-dev",
    ]);
});

test("tell dev builds from releases", () => {
    const versions = ["2026.9.0", "2026.9.0-dev", "2026.10.1-dev"];

    expect(versions.map((version) => Version.isPrerelease(version))).toEqual([false, true, true]);
});

test("order the development version after every release, and keep it through a dev build", () => {
    const dev = Version.dev(Version.development);
    expect([
        Version.development,
        Version.release(dev),
        Version.compare(Version.development, "2026.12.99"),
        Version.compare(dev, "2026.12.99"),
        Version.compare(dev, Version.development),
    ]).toEqual(["9999.12.0", "9999.12.0", 1, 1, -1]);
});
