import { expect, test } from "@destack/test";
import {
    DOMAINS,
    MachineAddress,
    InstallationOrigin,
    SpaceAddress,
    SpaceOrigin,
} from "../src/address/index.ts";

test("read the machine address of a server name, and nothing for other shapes", () => {
    expect(
        [
            "macbook--flotothemoon.destack.computer",
            "a--macbook--flotothemoon.destack.computer",
            "macbook.flotothemoon.destack.computer",
            "macbook--.destack.computer",
            "-macbook--flotothemoon.destack.computer",
            "notes--personal--flotothemoon.destack.space",
        ].map((name) => MachineAddress.parse(name, DOMAINS.machine)),
    ).toEqual([
        { machine: "macbook", handle: "flotothemoon" },
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});

test("read and write the origins of views and branches under a domain, and nothing for other hostnames", () => {
    const origins = [
        "notes--personal--florian.destack.space",
        "notes--feature-x---personal--florian.destack.space",
        "notes--personal--florian.localhost",
        "personal--florian.destack.space",
        "a--notes--personal--florian.destack.space",
        "notes.personal.florian.destack.space",
        "notes--feature-x---.florian.destack.space",
        "notes--Personal--florian.destack.space",
        "notes--personal--florian.example.com",
    ].map((hostname) =>
        InstallationOrigin.parse(
            hostname,
            hostname.endsWith("localhost") ? "localhost" : "destack.space",
        ),
    );
    expect(origins).toEqual([
        { name: "notes", space: "personal", handle: "florian" },
        { name: "notes", branch: "feature-x", space: "personal", handle: "florian" },
        { name: "notes", space: "personal", handle: "florian" },
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);

    // write back the hostnames the origins were read from
    const [, branched, local] = origins;
    if (branched === undefined || local === undefined) {
        throw new TypeError("the branch and local origins are unread");
    }
    expect([
        InstallationOrigin.format(branched, "destack.space"),
        InstallationOrigin.format(local, "localhost"),
    ]).toEqual([
        "notes--feature-x---personal--florian.destack.space",
        "notes--personal--florian.localhost",
    ]);
});

test("read and write the origins of spaces and branches under a domain, and nothing for installation origins or other hostnames", () => {
    const origins = [
        "personal--florian.destack.space",
        "feature-x---personal--florian.destack.space",
        "notes--personal--florian.destack.space",
        "feature-x---.florian.destack.space",
        "Personal--florian.destack.space",
        "personal--florian.example.com",
    ].map((hostname) => SpaceOrigin.parse(hostname, DOMAINS.space));
    expect(origins).toEqual([
        { space: "personal", handle: "florian" },
        { branch: "feature-x", space: "personal", handle: "florian" },
        undefined,
        undefined,
        undefined,
        undefined,
    ]);

    // write back the hostnames the origins were read from
    expect(
        origins.flatMap((origin) =>
            origin === undefined ? [] : [SpaceOrigin.format(origin, DOMAINS.space)],
        ),
    ).toEqual(["personal--florian.destack.space", "feature-x---personal--florian.destack.space"]);
});

test("fit the longest names of each part in one DNS label, and refuse longer names and names that would start a punycode label", () => {
    const longest = { name: "a".repeat(15), space: "b".repeat(20), handle: "c".repeat(24) };
    const machine = { machine: "d".repeat(37), handle: longest.handle };
    const hostname = InstallationOrigin.format(longest, DOMAINS.space);
    const server = MachineAddress.format(machine, DOMAINS.machine);

    // read back both 63-character labels and refuse one more character in any part
    expect([
        hostname.length - DOMAINS.space.length - 1,
        server.length - DOMAINS.machine.length - 1,
        InstallationOrigin.parse(hostname, DOMAINS.space),
        MachineAddress.parse(server, DOMAINS.machine),
        InstallationOrigin.parse(`a${hostname}`, DOMAINS.space),
        SpaceOrigin.parse(`b${longest.space}--florian.destack.space`, DOMAINS.space),
        SpaceOrigin.parse(`personal--c${longest.handle}.destack.space`, DOMAINS.space),
        MachineAddress.parse(`d${server}`, DOMAINS.machine),
        InstallationOrigin.parse("xn--personal--florian.destack.space", DOMAINS.space),
        SpaceOrigin.parse("xn--florian.destack.space", DOMAINS.space),
        MachineAddress.parse("xn--florian.destack.computer", DOMAINS.machine),
    ]).toEqual([
        63,
        63,
        longest,
        machine,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});

test("write a branch's label readable while it fits 63 characters, else by the digest of what overflows, and read both back", () => {
    const fitting = { name: "notes", branch: "feature-x", space: "personal", handle: "florian" };
    const longest = {
        name: "a".repeat(15),
        branch: "b".repeat(20),
        space: "c".repeat(20),
        handle: "d".repeat(24),
    };
    const labels = [
        InstallationOrigin.format(fitting, DOMAINS.space),
        InstallationOrigin.format(longest, DOMAINS.space),
        SpaceOrigin.format(longest, DOMAINS.space),
    ];

    // keep the fitting branch readable, and digest the name and branch, or the branch alone, at the longest names
    const tail = `---${longest.space}--${longest.handle}.destack.space`;
    expect(labels).toEqual([
        "notes--feature-x---personal--florian.destack.space",
        `5446050461${tail}`,
        `f467d0bb11${tail}`,
    ]);

    // read the readable branch back whole, and the digests as the labels write them
    expect([
        InstallationOrigin.parse(labels[0] ?? "", DOMAINS.space),
        InstallationOrigin.parse(labels[1] ?? "", DOMAINS.space),
        SpaceOrigin.parse(labels[2] ?? "", DOMAINS.space),
    ]).toEqual([
        fitting,
        { digest: "5446050461", space: longest.space, handle: longest.handle },
        { branch: "f467d0bb11", space: longest.space, handle: longest.handle },
    ]);
});

test("read space addresses as one label, refusing other shapes, and write them back from their origins", () => {
    // admit a space name and a handle joined by -- at their longest, and refuse every other shape
    const longest = `${"c".repeat(20)}--${"d".repeat(24)}`;
    const addresses = [
        "personal--florian",
        "work-2--acme",
        "xn-a--b-xn",
        longest,
        "personal.florian",
        "notes--personal--florian",
        "personal---florian",
        "-personal--florian",
        "Personal--florian",
        `c${longest}`,
        `${longest}d`,
        "xn--florian",
        "personal--xn",
    ];
    expect(addresses.map((address) => SpaceAddress.safeParse(address).success)).toEqual([
        true,
        true,
        true,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
    ]);

    // read the origin of an address and write the address of the origin with a branch
    const origin = SpaceAddress.origin("work-2--acme");
    expect([origin, SpaceAddress.of({ branch: "feature", ...origin })]).toEqual([
        { space: "work-2", handle: "acme" },
        "work-2--acme",
    ]);
});
