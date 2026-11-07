import { space } from "@destack/account/object";
import { expect, test, refusal } from "@destack/test";
import { principal, relation } from "@destack/access";
import { defineObject } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

import { Noun, Catalog } from "./catalog.ts";

/** The package declaring every type of the tests. */
const PACKAGE_ID = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000c1");

/** Declare an object type of a name and its identifiers' prefix. */
function declared(name: string, identity = name) {
    return defineObject({
        name,
        plural: `${name}s`,
        identity,
        scope: space,
        fields: {},
        permissions: { read: relation("owner") },
        relations: { owner: { subjects: [principal.user] } },
    });
}

/** A noun of an object type at an installation of a name, or a base installation of its package. */
function noun(installation: string, name: string, identity = name): Noun {
    const type = declared(name, identity);

    return new Noun(
        {
            id: schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-0000000000c2"),
            scope: "space-01996ab0-0000-7000-8000-0000000000c3",
            name: installation,
            packageId: PACKAGE_ID,
            types: [type.typeReference],
            declarations: [],
        },
        "space",
        type,
        [],
    );
}

test("find nouns by their words, qualifying the words several installations serve", async () => {
    const catalog = new Catalog(
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
    const nameOf = (word: string) => {
        const found = catalog.find(word);

        return found === undefined
            ? undefined
            : `${found.qualifier}:${found.name}${found.lists(word) ? " (plural)" : ""}`;
    };

    // resolve nouns one installation serves, and qualified nouns
    expect(
        ["installations", "account:machine", "notes:machines", "work:tasks", "notes", "page"].map(
            nameOf,
        ),
    ).toEqual([
        "space:installation (plural)",
        "account:machine",
        "notes:machine (plural)",
        "work:task (plural)",
        "notes:note (plural)",
        undefined,
    ]);

    // list each noun by its plural, qualified where another noun or a root verb takes a word of it
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

    // refuse a noun two installations serve, built-in or installed, listing the qualified nouns
    expect(await refusal(() => catalog.find("task"))).toEqual([
        "BAD_REQUEST",
        "USAGE_INVALID",
        "task is ambiguous: use one of work:task, home:task",
    ]);
    expect(await refusal(() => catalog.find("machines"))).toEqual([
        "BAD_REQUEST",
        "USAGE_INVALID",
        "machines is ambiguous: use one of account:machines, notes:machines",
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
        "several nouns list tasks: work:tasks, home:tasks",
    ]);
});
