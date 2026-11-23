import { expect, test } from "@destack/test";
import { Profile } from "@destack/update/release";
import { UniverseHome } from "./home.ts";

test("place each repository's checkout at its remote's host and path, without its port or .git suffix, a path at the universe", () => {
    const home = new UniverseHome(
        new Profile("local"),
        "http://destack.localhost:4100",
        "/Users/ada",
    );

    expect(
        [
            "https://destack.app/@destack/home.git",
            "http://destack.localhost:4100/@ada/apps.git",
            "https://forge.example/@ada/notes",
            "/@ada/notes.git",
        ].map((remote) => home.checkout(remote)),
    ).toEqual([
        "/Users/ada/Destack/destack.app/@destack/home",
        "/Users/ada/Destack/destack.localhost/@ada/apps",
        "/Users/ada/Destack/forge.example/@ada/notes",
        "/Users/ada/Destack/destack.localhost/@ada/notes",
    ]);
});
