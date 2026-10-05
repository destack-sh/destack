import { expect, test } from "@destack/test";
import { none, Policy } from "@destack/access";
import { defineObject } from "@destack/object";
import { PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { describeCommand, describeView } from "../inspect/index.ts";
import { defineCommand } from "./command.ts";
import { defineView } from "./view.ts";

/** The declaring package. */
const notes = {
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** The notes a view opens. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
    methods: (method) => ({ list: method.list("read") }),
});

test("describe a declared view by its name and permissions without loading its component", () => {
    // declare a view of notes whose component never loads
    const view = defineView(
        {
            name: "notes",
            objects: [note],
            permissions: [note.permission("read")],
            presents: [{ object: note, priority: "default" }],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    // describe the name, permissions and presented types, and keep the package and object types
    expect([view.package, view.objects, describeView(view)]).toEqual([
        notes,
        [note],
        {
            name: "notes",
            permissions: [{ packageId: note.package.id, type: "note", name: "read" }],
            presents: [{ packageId: note.package.id, type: "note", priority: "default" }],
        },
    ]);
});

test("describe the object types a view opens in the person's home, and refuse a type opened twice", () => {
    // declare a view reading notes in the person's home
    const view = defineView(
        {
            name: "inbox",
            home: [note],
            permissions: [note.permission("read")],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );
    const twice = () =>
        defineView(
            {
                name: "notes",
                objects: [note],
                home: [note],
                component: () => Promise.reject(new Error("loaded")),
            },
            { package: notes },
        );

    // name the home type beside its permission, and refuse the type in both lists
    expect(describeView(view).home).toEqual([{ packageId: note.package.id, type: "note" }]);
    expect(twice).toThrow(
        new TypeError("view notes opens note both in its scopes and in the home"),
    );
});

test("refuse a view presenting an object type it opens none of", () => {
    // present notes without opening them
    const refusal = () =>
        defineView(
            {
                name: "notes",
                presents: [{ object: note, priority: "default" }],
                component: () => Promise.reject(new Error("loaded")),
            },
            { package: notes },
        );
    expect(refusal).toThrow(new TypeError("view notes presents note but opens no note objects"));
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

test("refuse a view requesting a permission of an object type it opens none of", () => {
    // request reading notes without opening them
    const refusal = () =>
        defineView(
            {
                name: "notes",
                permissions: [note.permission("read")],
                component: () => Promise.reject(new Error("loaded")),
            },
            { package: notes },
        );
    expect(refusal).toThrow(
        new TypeError("view notes requests note read but opens no note objects"),
    );
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
        methods: (method) => ({ list: method.list("read") }),
    });
    const view = defineView(
        {
            name: "devices",
            objects: [device],
            permissions: [device.permission("read")],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );
    expect(view.permissions).toEqual([device.permission("read")]);
});
