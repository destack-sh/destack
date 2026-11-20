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

test("read app and repository links below the universe's origin, and nothing else", () => {
    expect(
        [
            "/@ada",
            "/@ada/",
            "/@ada/home/notes",
            "/@ada/home.git",
            "/@ada/home.git/info/refs",
            "/@ada/home.git/git-receive-pack",
            "/@ada/home",
            "/@ada/home/notes/edit",
            "/@ada/Home.git",
            "/ada",
            "/@A",
        ].map((path) => UniverseOrigin.link(path)),
    ).toEqual([
        undefined,
        undefined,
        { kind: "app", installation: { name: "notes", space: "home", handle: "ada" } },
        { kind: "repository", handle: "ada", name: "home", path: "" },
        { kind: "repository", handle: "ada", name: "home", path: "/info/refs" },
        { kind: "repository", handle: "ada", name: "home", path: "/git-receive-pack" },
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});
