import { expect, test } from "@destack/test";
import { PlatformDomain, UniverseOrigin } from "./address.ts";

test("cover the platform's domains and the hostnames and wildcards below them, but no account's own", () => {
    expect(
        [
            "destack.app",
            "*.destack.space",
            "relay-eu.destack.computer",
            "notes.acme.com",
            "destack.app.acme.com",
            "notdestack.app",
        ].map((hostname) => PlatformDomain.covers(hostname)),
    ).toEqual([true, true, true, false, false, false]);
});

test("read a profile link and an app link below the universe's origin, and nothing else", () => {
    expect(
        [
            "/@ada",
            "/@ada/",
            "/@ada/home/notes",
            "/@ada/home",
            "/@ada/home/notes/edit",
            "/ada",
            "/@A",
        ].map((path) => UniverseOrigin.link(path)),
    ).toEqual([
        { kind: "profile", handle: "ada" },
        { kind: "profile", handle: "ada" },
        { kind: "app", installation: { name: "notes", space: "home", handle: "ada" } },
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});
