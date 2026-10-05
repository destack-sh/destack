import { expect, test } from "@destack/test";
import { DOMAINS, HostAddress, InstallationOrigin, SpaceOrigin } from "../src/address/index.ts";

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
    const [, branched, local] = origins;
    if (branched === undefined || local === undefined) {
        throw new TypeError("the branch and device origins are unread");
    }
    expect([
        InstallationOrigin.format(branched, "destack.space"),
        InstallationOrigin.format(local, "localhost"),
    ]).toEqual([
        "notes.feature-x--personal.florian.destack.space",
        "notes.personal.florian.localhost",
    ]);
});

test("read and write the origins of spaces and branches under a domain, and nothing for installation origins or other hostnames", () => {
    const origins = [
        "personal.florian.destack.space",
        "feature-x--personal.florian.destack.space",
        "notes.personal.florian.destack.space",
        "feature-x--.florian.destack.space",
        "Personal.florian.destack.space",
        "personal.florian.example.com",
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
    ).toEqual(["personal.florian.destack.space", "feature-x--personal.florian.destack.space"]);
});
