import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { MachineOrigin, UniverseOrigin } from "./address.ts";

/** A machine's identifier. */
const MACHINE = schema.identifier("machine").parse("machine-01a0f4c2-c400-7425-abe7-d5f2cb80c824");

/** A universe at Destack's origin and domains. */
const UNIVERSE = new UniverseOrigin("https://destack.app", {
    space: "destack.space",
    machine: "destack.computer",
    site: "destack.sh",
});

test("cover the universe's domains and the hostnames and wildcards below them, but no account's own", () => {
    expect(
        [
            "destack.app",
            "*.destack.space",
            "relay-eu.destack.computer",
            "notes.example.com",
            "destack.app.example.com",
            "notdestack.app",
        ].map((hostname) => UNIVERSE.covers(hostname)),
    ).toEqual([true, true, true, false, false, false]);
});

test("write the URLs of a universe's names at its scheme and port, a loopback machine's below its own", () => {
    // address a local universe on a port, beside the one at Destack's origin
    const local = new UniverseOrigin("http://destack.localhost:4100", {
        space: "space.localhost",
        machine: "computer.localhost",
        site: "site.localhost",
    });
    const home = { name: "home", space: "home", handle: "ada" };

    expect({
        issuer: [UNIVERSE.issuer, local.issuer],
        page: [UNIVERSE.page("signIn").href, local.page("signIn").href],
        machine: [UNIVERSE.machine(MACHINE), local.machine(MACHINE)],
        installation: [
            UNIVERSE.installation(home, "https://eu-central--platform.destack.computer"),
            local.installation(home, "http://macbook--ada.computer.localhost:4100"),
            local.installation(home, "http://127.0.0.1:4101"),
        ],
        serves: [
            local.serves("http://destack.localhost:4100/oauth/oauth2/token"),
            local.serves("http://destack.localhost:4100/@ada/home.git"),
            local.serves("http://destack.localhost:4100/sign-in"),
            local.serves("http://ada.space.localhost:4100/oauth"),
        ],
        json: UniverseOrigin.of(UniverseOrigin.schema.parse(local.toJSON())),
    }).toEqual({
        issuer: ["https://destack.app/oauth", "http://destack.localhost:4100/oauth"],
        page: ["https://destack.app/sign-in", "http://destack.localhost:4100/sign-in"],
        machine: [
            "https://machine-01a0f4c2-c400-7425-abe7-d5f2cb80c824.destack.computer",
            "http://machine-01a0f4c2-c400-7425-abe7-d5f2cb80c824.computer.localhost:4100",
        ],
        installation: [
            "https://ada.destack.space",
            "http://ada.space.localhost:4100",
            "http://ada.space.localhost:4101",
        ],
        serves: ["issuer", "link", undefined, undefined],
        json: local,
    });
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

test("read a machine's origin back to the machine, and no machine from its name", () => {
    const origin = new URL(UNIVERSE.machine(MACHINE)).hostname;

    expect([
        MachineOrigin.parse(origin, "destack.computer"),
        MachineOrigin.parse("macbook--ada.destack.computer", "destack.computer"),
        MachineOrigin.parse(origin, "destack.space"),
    ]).toEqual([MACHINE, undefined, undefined]);
});
