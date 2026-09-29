import { expect, test } from "@destack/test";
import { DOMAINS, HostAddress, InstallationOrigin, SpaceAddress } from "../src/address/index.ts";

test("read the host address of a server name, and nothing for other shapes", () => {
    expect(
        [
            "macbook.flotothemoon.destack.computer",
            "a.macbook.flotothemoon.destack.computer",
            "macbook..destack.computer",
            "-macbook.flotothemoon.destack.computer",
            "notes.personal.flotothemoon.destack.space",
        ].map((name) => HostAddress.parse(name, DOMAINS.host)),
    ).toEqual([
        { host: "macbook", handle: "flotothemoon" },
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});

test("read and write the origins of views and branches under a domain, and nothing for other hostnames", () => {
    const origins = [
        "notes.personal.florian.destack.space",
        "notes.feature-x--personal.florian.destack.space",
        "notes.personal.florian.localhost",
        "personal.florian.destack.space",
        "a.notes.personal.florian.destack.space",
        "notes.feature-x--.florian.destack.space",
        "notes.Personal.florian.destack.space",
        "notes.personal.florian.example.com",
    ].map((hostname) =>
        InstallationOrigin.parse(
            hostname,
            hostname.endsWith("localhost") ? "localhost" : "destack.space",
        ),
    );
    expect(origins).toEqual([
        { alias: "notes", space: "personal", handle: "florian" },
        { alias: "notes", branch: "feature-x", space: "personal", handle: "florian" },
        { alias: "notes", space: "personal", handle: "florian" },
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);

    // write back the hostnames the origins were read from
    expect([
        InstallationOrigin.format(origins[1]!, "destack.space"),
        InstallationOrigin.format(origins[2]!, "localhost"),
    ]).toEqual([
        "notes.feature-x--personal.florian.destack.space",
        "notes.personal.florian.localhost",
    ]);
});

test("accept a space's address as its name then its account's handle", () => {
    expect(
        ["notes.florian", "notes", "Notes.florian", "no--tes.florian"].map(
            (address) => SpaceAddress.safeParse(address).success,
        ),
    ).toEqual([true, false, false, false]);
});
