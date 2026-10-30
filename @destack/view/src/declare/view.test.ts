import { expect, expectTypeOf, test } from "@destack/test";
import { none } from "@destack/access";
import { defineObject } from "@destack/object";
import { Package, PackageId } from "@destack/package";
import { account, space } from "@destack/account/object";
import { describeCommand, describeView } from "../inspect/index.ts";
import { defineCommand } from "./command.ts";
import { PackageHandle } from "@destack/object";
import { defineView } from "./view.ts";

/** The declaring package. */
const notes = Package.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes of a space a view opens. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: {},
    permissions: { read: none() },
});

/** The budgets of an account a view opens. */
const budget = defineObject({
    name: "budget",
    plural: "budgets",
    scope: account,
    fields: {},
    permissions: { read: none() },
});

test("describe a declared view by its name, presented types, categories and packages without loading its component", () => {
    // declare a view of notes calling a package, whose component never loads
    const counter = PackageHandle.of(notes, {});
    const view = defineView(
        {
            name: "notes",
            objects: [note],
            presents: [{ object: note, priority: "default" }],
            categories: ["home"],
            packages: [counter],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    // describe the name, presented types, categories and packages, and keep the package and opened types
    expect([view.package, view.objects, describeView(view)]).toEqual([
        notes,
        [note],
        {
            name: "notes",
            presents: [{ packageId: note.package.id, type: "note", priority: "default" }],
            categories: ["home"],
            packages: [notes.id],
        },
    ]);
});

test("open each object type in the scopes it lives in: space types in the space and home, account types in the account", () => {
    // read the space's notes and the account's budgets
    const view = defineView(
        {
            name: "inbox",
            objects: [note, budget],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    expect([view.opens("space"), view.opens("home"), view.opens("account")]).toEqual([
        [note],
        [note],
        [budget],
    ]);
});

test("describe a declared command by the method it calls and its key combination", () => {
    // declare a command listing notes on a shortcut
    const command = defineCommand(
        {
            name: "list-notes",
            title: "List notes",
            object: note,
            method: "list",
            keybinding: "mod+l",
        },
        { package: notes },
    );
    expect([command.package, describeCommand(command)]).toEqual([
        notes,
        {
            title: "List notes",
            packageId: note.package.id,
            type: "note",
            method: "list",
            keybinding: "mod+l",
        },
    ]);
});

test("type a command's method as one clients call, leaving package and installation methods out", () => {
    // declare archived notes with a client, an installation and a package method
    const archived = defineObject(
        {
            name: "archived",
            plural: "archived",
            scope: space,
            fields: {},
            permissions: { read: none() },
        },
        (method) => ({
            open: method.mutation({ permission: "read" }),
            restore: method.mutation({ permission: "read", visibility: "installation" }),
            purge: method.mutation({ permission: "read", visibility: "package" }),
        }),
    );
    expectTypeOf<Parameters<typeof defineCommand<typeof archived>>[0]["method"]>().toEqualTypeOf<
        "get" | "list" | "open"
    >();
});
