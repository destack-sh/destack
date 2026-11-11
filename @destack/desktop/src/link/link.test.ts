import type { Address } from "@destack/view/declare";
import { expect, test } from "@destack/test";
import { Stage } from "@destack/update/release";
import { Link } from "./link.ts";

/** The links of the stable desktop. */
const links = new Link("destack");

/** Read the view or object of a link, failing for neither. */
function parse(href: string): Address {
    const address = links.parse(href);
    if (address === undefined) {
        throw new TypeError(`link addresses nothing: '${href}'`);
    }

    return address;
}

/** The note of the object links. */
const NOTE = {
    packageId: "package-01996ab0-0000-7000-8000-000000000001",
    type: "note",
    scope: "space-01996ab0-0000-7000-8000-000000000002",
    id: "note-1",
};

/** The query with the note as JSON. */
const NOTE_QUERY = new URLSearchParams({ object: JSON.stringify(NOTE) }).toString();

test("read deep links into their view and location and write them back", () => {
    const addresses = [
        "destack://notes--personal--florian/home",
        "destack://notes--personal--florian/home/",
        "destack://tasks--work-2--acme/board/projects/42?filter=open&sort=due",
    ].map((href) => parse(href));

    // retain the location, with its query, relative to the view
    expect(addresses).toEqual([
        { space: "personal--florian", installation: "notes", view: "home", path: "/" },
        { space: "personal--florian", installation: "notes", view: "home", path: "/" },
        {
            space: "work-2--acme",
            installation: "tasks",
            view: "board",
            path: "/projects/42?filter=open&sort=due",
        },
    ]);
    expect(addresses.map((address) => links.format(address))).toEqual([
        "destack://notes--personal--florian/home",
        "destack://notes--personal--florian/home",
        "destack://tasks--work-2--acme/board/projects/42?filter=open&sort=due",
    ]);
});

test("read object links into their space, object and view and write them back", () => {
    const hrefs = [
        `destack://personal--florian/.destack/open?${NOTE_QUERY}`,
        `destack://personal--florian/.destack/open?${NOTE_QUERY}&view=editor`,
    ];
    const addresses = hrefs.map((href) => parse(href));

    // retain the object, and the view asked for
    expect(addresses).toEqual([
        { space: "personal--florian", object: NOTE },
        { space: "personal--florian", object: NOTE, view: "editor" },
    ]);
    expect(addresses.map((address) => links.format(address))).toEqual(hrefs);
});

test("read nothing from foreign schemes and malformed addresses", () => {
    const hrefs = [
        "not a url",
        "https://notes--personal--florian/home",
        "destack-local://notes--personal--florian/home",
        "destack://user:secret@notes--personal--florian/home",
        "destack://notes--personal--florian:8080/home",
        "destack://personal--florian/home",
        "destack://notes--a--b--c/home",
        "destack://notes---personal--florian/home",
        "destack://notes--per----sonal--florian/home",
        "destack://Notes--personal--florian/home",
        "destack://notes--personal--florian",
        "destack://notes--personal--florian/1home",
        "destack://notes--personal--florian/home//evil.test",
        `destack://notes--personal--florian/.destack/open?${NOTE_QUERY}`,
        `destack://personal--florian/open?${NOTE_QUERY}`,
        "destack://personal--florian/.destack/open?object=note-1",
        "destack://personal--florian/.destack/open",
        `destack://personal--florian/.destack/open?${NOTE_QUERY}&view=1editor`,
    ];

    expect(hrefs.map((href) => links.parse(href))).toEqual(hrefs.map(() => undefined));
});

test("route each stage by its scheme", () => {
    // read and write local links only on the local stage
    const local = Link.of(new Stage("local"));
    const href = "destack-local://notes--personal--florian/home";
    const address = local.parse(href);
    expect({
        scheme: local.scheme,
        written: address === undefined ? undefined : local.format(address),
        stable: local.parse("destack://notes--personal--florian/home"),
    }).toEqual({ scheme: "destack-local", written: href, stable: undefined });
});

test("read the view a location shows beside an origin, and nothing on another scheme, port or domain", () => {
    // navigate from a space's origin and from a view's origin
    const space = new URL("http://personal--florian.localhost:7470/.destack/open");
    const notes = new URL("http://notes--personal--florian.localhost:7470/home");
    const navigations: readonly (readonly [URL, string])[] = [
        [space, "http://notes--personal--florian.localhost:7470/editor?token=code&target=x"],
        [notes, "http://tasks--personal--florian.localhost:7470/board"],
        [notes, "https://tasks--personal--florian.localhost:7470/board"],
        [notes, "http://tasks--personal--florian.localhost:9999/board"],
        [notes, "https://example.com/"],
    ];

    // read a view beside either origin, without the launch token
    expect(navigations.map(([origin, href]) => Link.beside(origin, new URL(href)))).toEqual([
        { space: "personal--florian", installation: "notes", view: "editor", path: "/?target=x" },
        { space: "personal--florian", installation: "tasks", view: "board", path: "/" },
        undefined,
        undefined,
        undefined,
    ]);
});
