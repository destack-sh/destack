import { expect, test } from "@destack/test";
import { none } from "@destack/access";
import { defineObject } from "@destack/object";
import { PackageVersion, PackageId } from "@destack/package";
import { account, space } from "@destack/account/object";
import { describeView } from "../inspect/index.ts";
import { Package } from "@destack/object";
import { defineView } from "./view.ts";
import { argument, plural, t } from "@destack/locale";

/** The declaring package. */
const notes = PackageVersion.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes of a space a view opens. */
const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
    scope: space,
    fields: {},
    access: {
        permissions: { read: none() },
    },
});

/** The budgets of an account a view opens. */
const budget = defineObject({
    name: "budget",
    plural: "budgets",
    title: t`${plural(argument("count"), { one: "Budget", other: "Budgets" })}`,
    scope: account,
    fields: {},
    access: {
        permissions: { read: none() },
    },
});

test("describe a declared view by its name, presented types, categories, packages and render mode without loading its component", () => {
    // declare a view of notes calling a package, whose component never loads
    const counter = Package.of(notes, {});
    const view = defineView(
        {
            name: "notes",
            title: t`Notes`,
            objects: [note],
            presents: [{ object: note, priority: "default" }],
            categories: ["home"],
            audience: "person",
            packages: [counter],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    // describe the name, presented types, categories, packages and the client mode it renders in by default, and keep the package and opened types
    expect([view.package, view.objects, describeView(view)]).toEqual([
        notes,
        [note],
        {
            name: "notes",
            presents: [{ packageId: note.package.id, type: "note", priority: "default" }],
            categories: ["home"],
            audience: "person",
            packages: [notes.id],
            render: { mode: "client" },
        },
    ]);
});

test("open each object type in the scopes it lives in: space types in the space and home, account types in the account", () => {
    // read the space's notes and the account's budgets
    const view = defineView(
        {
            name: "inbox",
            title: t`Inbox`,
            objects: [note, budget],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    expect([view.opens(space), view.opens(account)]).toEqual([[note], [budget]]);
});
