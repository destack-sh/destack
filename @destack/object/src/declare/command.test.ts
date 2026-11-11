import { none, principal } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import { argument, plural, t } from "@destack/locale";
import { PackageVersion, PackageId } from "@destack/package";
import { Identifier, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, expectTypeOf, onTestFinished, refusal, test } from "@destack/test";
import { ObjectClient } from "../client/index.ts";
import { testSimulation } from "@destack/test/world";
import { field } from "../field/index.ts";
import { defineObject, device } from "../object/index.ts";
import type { ShortcutText } from "./shortcut.ts";
import { Command, type CommandDefinition, defineCommand } from "./command.ts";
import { definePackage } from "./package.ts";
import { recoverable } from "../index.ts";

/** The declaring package. */
const notes = PackageVersion.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
});

/** The notes of a space, which open, mark, archive and purge. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        scope: device,
        fields: {
            title: field.string(schema.string()),
            status: field.enum(["open", "done"]).default("open"),
            priority: field.integer().default(0),
        },
        access: {
            permissions: { read: none(), update: none(), create: none() },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["title"] }),
        open: method.mutation({ title: t`Open`, permission: "read" }),
        mark: method.update("update", { fields: ["status", "priority"] }),
        delete: method.delete("update"),
        restore: method.mutation({
            title: t`Restore`,
            permission: "read",
            visibility: "installation",
        }),
        purge: method.mutation({ title: t`Purge`, permission: "read", visibility: "package" }),
    }),
);

/** The drafts of a device, which delete to a trash for 30 days. */
const draft = defineObject(
    {
        name: "draft",
        plural: "drafts",
        title: t`${plural(argument("count"), { one: "Draft", other: "Drafts" })}`,
        scope: device,
        fields: { title: field.string(schema.string()) },
        access: {
            permissions: { update: none() },
        },
        traits: [recoverable({ within: { days: 30 }, by: "update" })],
    },
    () => ({}),
);

/** The tasks of a device, stored locally, which a command marks. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: device,
        storage: "local",
        fields: { status: field.enum(["open", "done"]) },
        access: {
            permissions: { create: none(), update: none() },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["status"] }),
        mark: method.update("update", { fields: ["status"] }),
    }),
);

test("describe commands by the call each makes, and read them back from their descriptions", () => {
    // declare a command marking a note done under a category, and one creating notes on a shortcut
    const done = defineCommand(
        {
            name: "mark-done",
            title: t`Mark Done`,
            category: t`Note`,
            object: note,
            method: "mark",
            input: { status: "done" },
            when: { status: "open" },
        },
        { package: notes },
    );
    const create = defineCommand(
        {
            name: "new-note",
            title: t`New Note`,
            object: note,
            method: "create",
            keybinding: "mod+n",
        },
        { package: notes },
    );

    // describe each, then read each back from its description
    const described = [done, create].map((command) => command.describe());
    const read = [done, create].map((command) =>
        Command.of(command.name, command.describe(), note).describe(),
    );

    // expect the calls, the preset input, the condition and the keys, the same once read back
    const call = { package: notes, packageId: note.package.id, type: "note" };
    expect({ package: done.package, described, read }).toEqual({
        package: notes,
        described: [
            {
                ...call,
                name: "mark-done",
                title: t`Mark Done`,
                category: t`Note`,
                method: "mark",
                input: { status: "done" },
                when: { status: "open" },
            },
            {
                ...call,
                name: "new-note",
                title: t`New Note`,
                method: "create",
                input: {},
                keybinding: "mod+n",
            },
        ],
        read: described,
    });
});

test("type a command's method as one clients call, its preset as the method's fields, its input as the rest and its keys by their syntax", () => {
    // declare a command presetting a note's status
    const done = defineCommand(
        {
            name: "mark-done",
            title: t`Mark Done`,
            object: note,
            method: "mark",
            input: { status: "done" },
        },
        { package: notes },
    );

    // expect the methods clients call, and the input without the preset status
    expectTypeOf<CommandDefinition<typeof note>["method"]>().toEqualTypeOf<
        "get" | "list" | "create" | "open" | "mark" | "delete" | "attach" | "detach"
    >();
    expectTypeOf<Parameters<typeof done.input>[0]>().toHaveProperty("id");
    expectTypeOf<Parameters<typeof done.input>[0]>().not.toHaveProperty("status");

    // expect a misspelled preset field typed never, and keys checked against the keybinding syntax
    type Misspelled = NonNullable<
        CommandDefinition<typeof note, "mark", { stauts: "done" }>["input"]
    >;
    expectTypeOf<Misspelled["stauts"]>().toBeNever();
    expectTypeOf<ShortcutText<"mod+shift+k">>().toEqualTypeOf<"mod+shift+k">();
    expectTypeOf<ShortcutText<"hyper+x">>().toBeNever();
    expectTypeOf<ShortcutText<"mod+">>().toBeNever();
});

test("read what a command acts on, the input it asks for and whether it removes its target", () => {
    // declare commands marking, creating and deleting notes, and one moving a draft to the trash
    const owner = { package: notes };
    const done = defineCommand(
        {
            name: "mark-done",
            title: t`Mark Done`,
            object: note,
            method: "mark",
            input: { status: "done" },
        },
        owner,
    );
    const create = defineCommand(
        { name: "new-note", title: t`New Note`, object: note, method: "create" },
        owner,
    );
    const remove = defineCommand(
        { name: "delete-note", title: t`Delete Note`, object: note, method: "delete" },
        owner,
    );
    const trash = defineCommand(
        { name: "trash-draft", title: t`Move to Trash`, object: draft, method: "delete" },
        owner,
    );

    // expect each on one object but the create, asking for the rest, the note's delete alone for good
    expect([done, create, remove, trash].map(readCommand)).toEqual([
        { target: "object", asked: ["priority", "tags"], isDestructive: false },
        { target: "scope", asked: ["title", "tags"], isDestructive: false },
        { target: "object", asked: [], isDestructive: true },
        { target: "object", asked: [], isDestructive: false },
    ]);
});

test("query the targets a command applies to and write its call's input from the preset fields", async () => {
    // store a device's tasks locally, one open and one done
    const environment = testSimulation().environment("device");
    const database = await connectBunSqlite(":memory:", [], { environment });
    onTestFinished(() => database.close());
    const owner = principal.device.reference(
        Scope.universe.id,
        Identifier.create("device", environment),
    );
    const client = await ObjectClient.open({
        environment,
        database,
        objects: { task },
        scope: owner.id,
        caller: owner,
        device: owner,
        endpoint: {
            url: "http://local.invalid/",
            fetch: async () => {
                throw new TypeError("local tasks call no server");
            },
        },
        reconnect: () => {
            throw new TypeError("local tasks move to no machine");
        },
    });
    onTestFinished(() => client.close());
    const open = await client.mutate(task).create({ status: "open" }).predicted;
    const done = await client.mutate(task).create({ status: "done" }).predicted;

    // mark the open task done as a command, reading where it applies before and after
    const markDone = defineCommand(
        {
            name: "mark-done",
            title: t`Mark Done`,
            object: task,
            method: "mark",
            input: { status: "done" },
            when: { status: "open" },
        },
        { package: notes },
    );
    const applies = async (id: string) =>
        (await client.queryType(task).findFirst(markDone.query([id]))) !== undefined;
    const before = [await applies(open.id), await applies(done.id)];
    await client.call(task, markDone.method, markDone.input({ id: open.id })).confirmed;

    // expect the query of the method's permission, applying to the open task until it is done
    expect({
        query: markDone.query([open.id]),
        before,
        after: await applies(open.id),
    }).toEqual({
        query: {
            where: { AND: [{ id: { in: [open.id] } }, { status: "open" }] },
            permission: "update",
        },
        before: [true, false],
        after: false,
    });
});

test("refuse a condition on a command without a target, and a key combination editors do not write", async () => {
    // declare a create with a condition, and read back a command on keys the compiler cannot check
    const owner = { package: notes };
    const open = defineCommand(
        { name: "open-note", title: t`Open`, object: note, method: "open" },
        owner,
    );
    const refused = [
        await refusal(() =>
            defineCommand(
                {
                    name: "new-note",
                    title: t`New Note`,
                    object: note,
                    method: "create",
                    when: { status: "open" },
                },
                owner,
            ),
        ),
        await refusal(() =>
            Command.of("open-note", { ...open.describe(), keybinding: "hyper+o" }, note),
        ),
    ];

    // expect both refused, the condition by its method and the keys by their syntax
    expect(refused).toEqual([
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "command 'new-note' is invalid: it has a condition, but 'note.create' acts on its scope",
        ],
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "key combination 'hyper+o' of command 'open-note' is invalid: editors do not write it",
        ],
    ]);
});

test("list a package's commands on its own objects, refusing one on a type it does not serve or named as a method", async () => {
    // declare commands on tasks, on notes and one named as the tasks' mark method
    const owner = { package: task.package };
    const markDone = defineCommand(
        {
            name: "mark-done",
            title: t`Mark Done`,
            object: task,
            method: "mark",
            input: { status: "done" },
        },
        owner,
    );
    const stray = defineCommand(
        { name: "finish", title: t`Finish`, object: note, method: "mark" },
        owner,
    );
    const named = defineCommand(
        { name: "mark", title: t`Mark`, object: task, method: "mark" },
        owner,
    );

    // list each on a package serving the tasks alone
    const handle = definePackage({ objects: [task], commands: [markDone] }, owner);
    const refused = [
        await refusal(() => definePackage({ objects: [task], commands: [stray] }, owner)),
        await refusal(() => definePackage({ objects: [task], commands: [named] }, owner)),
    ];

    // expect the command listed, and both others refused by name
    expect({ listed: Object.keys(handle.commands), refused }).toEqual({
        listed: ["mark-done"],
        refused: [
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                `command 'finish' is invalid: '${task.package.name}' lists it on note, which it does not serve`,
            ],
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                "command 'mark' is invalid: it takes the name of a method of task",
            ],
        ],
    });
});

/** Read what a command acts on, the input fields it asks for and whether it removes for good. */
function readCommand(command: Command) {
    const { properties } = schema
        .looseObject({ properties: schema.jsonObject() })
        .parse(command.parameters);

    return {
        target: command.target,
        asked: Object.keys(properties),
        isDestructive: command.isDestructive,
    };
}
