import { expect, test } from "@destack/test";
import { none, Policy } from "@destack/access";
import { defineObject } from "@destack/object";
import { PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { describeCommand, describeView } from "../inspect/index.ts";
import { defineCommand } from "./command.ts";
import { defineService } from "@destack/service";
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

/** The reminders a view opens in the person's home. */
const reminder = defineObject({
    name: "reminder",
    plural: "reminders",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
    methods: (method) => ({ list: method.list("read") }),
});

test("describe a declared view by its name, permissions, presented types and platform services without loading its component", () => {
    // declare a view of notes calling a platform service, whose component never loads
    const counter = defineService("counter", {}, { package: notes });
    const view = defineView(
        {
            name: "notes",
            objects: [note],
            permissions: { space: [note.permission("read")] },
            presents: [{ object: note, priority: "default" }],
            services: [counter],
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );

    // describe the name, permissions, presented types and services, and keep the package and object types
    expect([view.package, view.objects, describeView(view)]).toEqual([
        notes,
        [note],
        {
            name: "notes",
            permissions: { space: [{ packageId: note.package.id, type: "note", name: "read" }] },
            presents: [{ packageId: note.package.id, type: "note", priority: "default" }],
            services: [notes.id],
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

test("refuse a view requesting one permission in two scopes", () => {
    // read notes in the view's space and in the person's home
    const refusal = () =>
        defineView(
            {
                name: "notes",
                objects: [note],
                permissions: { space: [note.permission("read")], home: [note.permission("read")] },
                component: () => Promise.reject(new Error("loaded")),
            },
            { package: notes },
        );
    expect(refusal).toThrow(new TypeError("view notes requests note read in more than one scope"));
});

test("refuse a view opening an object type it requests no permission on", () => {
    // open notes without requesting a permission on them
    const refusal = () =>
        defineView(
            {
                name: "notes",
                objects: [note],
                component: () => Promise.reject(new Error("loaded")),
            },
            { package: notes },
        );
    expect(refusal).toThrow(new TypeError("view notes opens note but requests no note permission"));
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
                permissions: { account: [note.permission("read")] },
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
            permissions: { account: [device.permission("read")] },
            component: () => Promise.reject(new Error("loaded")),
        },
        { package: notes },
    );
    expect(view.opens("account")).toEqual([device]);
});
