import { space } from "@destack/account/object";
import { expect, test, refusal } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineObject } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

import { Noun, Catalog } from "./catalog.ts";
import { argument, plural, t } from "@destack/locale";

/** The package declaring every type of the tests. */
const PACKAGE_ID = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c1");

/** Declare an object type of a name and its identifiers' prefix. */
function declared(name: string, identity = name) {
    return defineObject({
        name,
        plural: `${name}s`,
        title: t`${plural(argument("count"), { one: "Record", other: "Records" })}`,
        identity,
        scope: space,
        fields: {},
        access: {
            permissions: { read: relation("owner") },
            relations: { owner: { subjects: [principal.user] } },
        },
    });
}

/** A noun of an object type at an installation of a name. */
function noun(installation: string, name: string, identity = name): Noun {
    const type = declared(name, identity);

    return new Noun(
        {
            packageId: PACKAGE_ID,
            scope: "space-01996ab0-0000-7000-8000-0000000000c3",
            installation: {
                id: schema
                    .identifier("installation")
                    .parse("installation-01996ab0-0000-7000-8000-0000000000c2"),
                name: installation,
            },
            types: [type.typeReference],
            declarations: [],
        },
        "space",
        type,
        [],
    );
}

/** Build a catalog where two installations serve tasks and two serve machines. */
function ambiguousCatalog(): Catalog {
    return new Catalog(
        [
            noun("space", "installation"),
            noun("account", "machine"),
            noun("work", "task"),
            noun("home", "task"),
            noun("notes", "note"),
            noun("notes", "machine"),
        ],
        [],
    );
}

test("find nouns by singular, plural and qualified words", () => {
    // resolve nouns one installation serves, and qualified nouns
    const catalog = ambiguousCatalog();
    const resolved = [
        "installations",
        "account:machine",
        "notes:machines",
        "work:tasks",
        "notes",
        "page",
    ].map((word) => {
        const found = catalog.find(word);

        return found === undefined
            ? undefined
            : `${found.qualifier}:${found.name}${found.lists(word) ? " (plural)" : ""}`;
    });
    expect(resolved).toEqual([
        "space:installation (plural)",
        "account:machine",
        "notes:machine (plural)",
        "work:task (plural)",
        "notes:note (plural)",
        undefined,
    ]);
});

test("qualify plurals that collide with another noun or a verb", () => {
    // list each noun by its plural, qualified where another noun or a root verb takes a word of it
    const catalog = ambiguousCatalog();
    expect([catalog.plurals([]), catalog.plurals(["note"])]).toEqual([
        [
            "installations",
            "account:machines",
            "work:tasks",
            "home:tasks",
            "notes",
            "notes:machines",
        ],
        [
            "installations",
            "account:machines",
            "work:tasks",
            "home:tasks",
            "notes:notes",
            "notes:machines",
        ],
    ]);
});

test("refuse a noun two installations serve, listing qualified nouns", async () => {
    // refuse a singular and a plural, each listing the qualified nouns
    const catalog = ambiguousCatalog();
    expect([
        await refusal(() => catalog.find("task")),
        await refusal(() => catalog.find("machines")),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "noun 'task' is invalid: it is ambiguous, so use one of 'home:task, work:task'",
        ],
        [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "noun 'machines' is invalid: it is ambiguous, so use one of 'account:machines, notes:machines'",
        ],
    ]);
});

test("find an object's noun by the longest prefix of its identifier", () => {
    const catalog = new Catalog(
        [noun("space", "installation"), noun("space", "revision", "installation-revision")],
        [],
    );

    // take the longer prefix before the shorter one
    expect(
        [
            "installation-revision-01996ab0-0000-7000-8000-000000000001",
            "installation-01996ab0-0000-7000-8000-000000000001",
            "task-01996ab0-0000-7000-8000-000000000001",
        ].map((id) => catalog.owner(id)?.name),
    ).toEqual(["revision", "installation", undefined]);
});

test("find the noun of an identity, taking the type of its prefix over types borrowing it", async () => {
    const catalog = new Catalog(
        [
            noun("account", "machine"),
            noun("machine", "computer", "machine"),
            noun("work", "task"),
            noun("home", "task"),
        ],
        [],
    );

    // take the machine over the computer, and refuse two types of one name
    expect([catalog.identity("machine")?.qualifier, catalog.identity("note")]).toEqual([
        "account",
        undefined,
    ]);
    expect(await refusal(() => catalog.identity("task"))).toEqual([
        "BAD_REQUEST",
        "USAGE_INVALID",
        "'task' is invalid: several nouns list it as 'work:tasks, home:tasks'",
    ]);
});
