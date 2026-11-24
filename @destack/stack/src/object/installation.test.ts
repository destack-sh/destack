import { expect, test } from "@destack/test";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { Installation } from "./installation.ts";

/** The fixture packages' local names, whose positions number their identifiers. */
const NAMES = ["tasks", "contacts", "vault", "search", "notes", "left", "right"];

/** Number a fixture package's identifiers by its local name's position. */
function numbered(name: string): string {
    return `01996ab0-0000-7000-8000-${String(NAMES.indexOf(name) + 1).padStart(12, "0")}`;
}

/** A package at a release, by its local name. */
function version(name: string): PackageVersion {
    return PackageVersion.parse({
        id: `package-${numbered(name)}`,
        name: `@example/${name}`,
        version: "2026.11.0",
    });
}

/** An installation of a package by its local name, chosen by someone or not. */
function installed(name: string, isRoot: boolean) {
    const id = schema.identifier("installation").parse(`installation-${numbered(name)}`);

    return { id, name, packageId: version(name).id, isRoot };
}

test("keep what roots require transitively, name what is missing, and drop what nothing reaches", () => {
    // tasks, a root, requires contacts, which requires vault; an old unreached search stays from before
    const tasks = installed("tasks", true);
    const contacts = installed("contacts", false);
    const search = installed("search", false);
    const requirements = [
        { by: tasks, package: version("contacts") },
        { by: contacts, package: version("vault") },
    ];

    // resolve from tasks
    const { required, missing, unrequired } = Installation.resolve(
        [tasks, contacts, search],
        requirements,
    );

    // require contacts and through it vault, which the space lacks, and leave search unrequired
    expect({
        required: [...required.keys()],
        missing: missing.map((each) => each.package.name),
        unrequired: unrequired.map((each) => each.name),
    }).toEqual({
        required: [version("contacts").id, version("vault").id],
        missing: ["@example/vault"],
        unrequired: ["search"],
    });
});

test("keep a root nothing requires, and drop non-roots that only require each other", () => {
    // a root nobody requires, and two non-roots requiring each other with no root reaching them
    const notes = installed("notes", true);
    const left = installed("left", false);
    const right = installed("right", false);
    const requirements = [
        { by: left, package: version("right") },
        { by: right, package: version("left") },
    ];

    // resolve them
    const { unrequired } = Installation.resolve([notes, left, right], requirements);

    // drop both non-roots, as a cycle among them is no root
    expect(unrequired.map((each) => each.name)).toEqual(["left", "right"]);
});
