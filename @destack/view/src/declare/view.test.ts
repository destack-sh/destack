import { expect, test } from "@destack/test";
import { none, Policy } from "@destack/access";
import { defineObject } from "@destack/object";
import { Package, PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { describeCommand, describeView } from "../inspect/index.ts";
import { defineCommand } from "./command.ts";
import { PackageHandle } from "@destack/runtime";
import { defineView } from "./view.ts";

/** The declaring package. */
const notes = Package.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes a view opens. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
});

/** The reminders a view opens in the person's home. */
const reminder = defineObject({
    name: "reminder",
    plural: "reminders",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
});

test("describe a declared view by its name, permissions, presented types and packages without loading its component", () => {
    // declare a view of notes calling a package, whose component never loads
    const counter = PackageHandle.of(notes, {});
    const view = defineView(
        {
            name: "notes",
            objects: [note],
            permissions: { space: [note.permission("read")] },
            presents: [{ object: note, priority: "default" }],
            packages: [counter],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    // describe the name, permissions, presented types and packages, and keep the package and object types
    expect([view.package, view.objects, describeView(view)]).toEqual([
        notes,
        [note],
        {
            name: "notes",
            permissions: { space: [{ packageId: note.package.id, type: "note", name: "read" }] },
            presents: [{ packageId: note.package.id, type: "note", priority: "default" }],
            packages: [notes.id],
        },
    ]);
});

test("open each object type in the scopes the view requests its permissions in", () => {
    // read notes in the view's space and reminders in the person's home
    const view = defineView(
        {
            name: "inbox",
            objects: [note, reminder],
            permissions: {
                space: [note.permission("read")],
                home: [reminder.permission("read")],
            },
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );
    expect([view.opens("space"), view.opens("home"), view.opens("account")]).toEqual([
        [note],
        [reminder],
        [],
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

test("accept a view requesting a permission of an opened object representing another package's policy", () => {
    // open devices whose access another package declares
    const devices = new Policy(
        {
            id: PackageId.parse("package-019f7480-0000-7000-8000-000000000002"),
            name: "@example/access",
            version: "2026.9.0",
        },
        { name: "device", relations: {}, permissions: { read: none() } },
    );
    const device = defineObject({
        name: "device",
        plural: "devices",
        scope: Scope.universe.id,
        isScope: true,
        represents: devices,
        fields: {},
        permissions: devices.definition.permissions,
    });
    const view = defineView(
        {
            name: "devices",
            objects: [device],
            permissions: { account: [device.permission("read")] },
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );
    expect(view.opens("account")).toEqual([device]);
});
