import { testIdentity } from "../test/index.ts";
import { none, principal } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import { argument, plural, t } from "@destack/locale";
import { PackageVersion, PackageId } from "@destack/package";
import { Identifier, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, expectTypeOf, onTestFinished, refusal, test } from "@destack/test";
import { ObjectClient, ObjectRemote } from "../client/index.ts";
import { testSimulation } from "../test/world.ts";
import { field } from "../field/index.ts";
import { defineObject, device } from "../object/index.ts";
import type { ShortcutText } from "./shortcut.ts";
import { Action, type ActionCallDefinition, defineAction } from "./action.ts";
import { definePackage } from "./package.ts";
import { controlled, recoverable } from "../index.ts";

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

/** The jobs of a device, which a controller removes once deleted. */
const job = defineObject(
    {
        name: "job",
        plural: "jobs",
        title: t`${plural(argument("count"), { one: "Job", other: "Jobs" })}`,
        scope: device,
        fields: { title: field.string(schema.string()) },
        access: {
            permissions: { delete: none() },
        },
        traits: [controlled()],
    },
    (method) => ({ delete: method.delete("delete") }),
);

/** The tasks of a device, stored locally, which an action marks. */
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

test("describe actions by their keys and the call each makes, and read them back from their descriptions", () => {
    // declare an action marking a note done under a category, one creating notes on a shortcut, and one a view handles
    const done = defineAction(
        {
            name: "mark-done",
            title: t`Mark Done`,
            category: t`Note`,
            call: {
                object: note,
                method: "mark",
                input: { status: "done" },
                when: { status: "open" },
            },
        },
        { package: notes },
    );
    const create = defineAction(
        {
            name: "new-note",
            title: t`New Note`,
            keybinding: "mod+n",
            call: { object: note, method: "create" },
        },
        { package: notes },
    );

    const pin = defineAction(
        { name: "pin", title: t`Pin`, keybinding: "mod+p", focus: "any" },
        { package: notes },
    );

    // describe each, then read each back from its description
    const described = [done, create, pin].map((action) => action.describe());
    const read = [done, create, pin].map((action) =>
        Action.of(
            action.name,
            action.describe(),
            action.call === undefined ? undefined : note,
        ).describe(),
    );

    // expect the calls, the preset input, the condition, the keys and the focus, the same once read back
    const called = { packageId: note.package.id, type: "note" };
    expect({ package: done.package, described, read }).toEqual({
        package: notes,
        described: [
            {
                package: notes,
                name: "mark-done",
                title: t`Mark Done`,
                category: t`Note`,
                focus: "page",
                call: {
                    ...called,
                    method: "mark",
                    input: { status: "done" },
                    when: { status: "open" },
                },
            },
            {
                package: notes,
                name: "new-note",
                title: t`New Note`,
                keybinding: "mod+n",
                focus: "page",
                call: { ...called, method: "create", input: {} },
            },
            { package: notes, name: "pin", title: t`Pin`, keybinding: "mod+p", focus: "any" },
        ],
        read: described,
    });
});

test("type an action's method as one clients call, its preset as the method's fields, its input as the rest and its keys by their syntax", () => {
    // declare an action presetting a note's status
    const done = defineAction(
        {
            name: "mark-done",
            title: t`Mark Done`,
            call: { object: note, method: "mark", input: { status: "done" } },
        },
        { package: notes },
    );

    // expect the methods clients call, and the input without the preset status
    expectTypeOf<ActionCallDefinition<typeof note>["method"]>().toEqualTypeOf<
        "get" | "list" | "create" | "open" | "mark" | "delete" | "attach" | "detach"
    >();
    type Input = Parameters<NonNullable<typeof done.call>["input"]>[0];
    expectTypeOf<Input>().toHaveProperty("id");
    expectTypeOf<Input>().not.toHaveProperty("status");

    // expect a misspelled preset field typed never, and keys checked against the keybinding syntax
    type Misspelled = NonNullable<
        ActionCallDefinition<typeof note, "mark", { stauts: "done" }>["input"]
    >;
    expectTypeOf<Misspelled["stauts"]>().toBeNever();
    expectTypeOf<ShortcutText<"mod+shift+k">>().toEqualTypeOf<"mod+shift+k">();
    expectTypeOf<ShortcutText<"hyper+x">>().toBeNever();
    expectTypeOf<ShortcutText<"mod+">>().toBeNever();
});

test("read what an action's call acts on, the input it asks for and whether it removes its target", () => {
    // declare actions marking, creating and deleting notes, one moving a draft to the trash, one deleting a job
    const owner = { package: notes };
    const done = defineAction(
        {
            name: "mark-done",
            title: t`Mark Done`,
            call: { object: note, method: "mark", input: { status: "done" } },
        },
        owner,
    );
    const create = defineAction(
        { name: "new-note", title: t`New Note`, call: { object: note, method: "create" } },
        owner,
    );
    const remove = defineAction(
        { name: "delete-note", title: t`Delete Note`, call: { object: note, method: "delete" } },
        owner,
    );
    const trash = defineAction(
        { name: "trash-draft", title: t`Move to Trash`, call: { object: draft, method: "delete" } },
        owner,
    );
    const cancel = defineAction(
        { name: "cancel-job", title: t`Cancel Job`, call: { object: job, method: "delete" } },
        owner,
    );

    // expect each on one object but the create, asking for the rest, the deletes without a trash for good
    expect([done, create, remove, trash, cancel].map(readCall)).toEqual([
        { target: "object", asked: ["priority", "tags"], isDestructive: false },
        { target: "scope", asked: ["title", "tags"], isDestructive: false },
        { target: "object", asked: [], isDestructive: true },
        { target: "object", asked: [], isDestructive: false },
        { target: "object", asked: [], isDestructive: true },
    ]);
});

test("query the targets an action's call applies to and write its input from the preset fields", async () => {
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
        remote: ObjectRemote.local(() => new TypeError("local tasks call no server")),
    });
    onTestFinished(() => client.close());
    const open = await client.mutate(task).create({ status: "open" }).predicted;
    const done = await client.mutate(task).create({ status: "done" }).predicted;

    // mark the open task done through an action, reading where it applies before and after
    const markDone = defineAction(
        {
            name: "mark-done",
            title: t`Mark Done`,
            call: {
                object: task,
                method: "mark",
                input: { status: "done" },
                when: { status: "open" },
            },
        },
        { package: notes },
    );
    const call = present(markDone.call, "the action's call");
    const applies = async (id: string) =>
        (await client.queryType(task).findFirst(call.query([id]))) !== undefined;
    const before = [await applies(open.id), await applies(done.id)];
    await client.call(task, call.method, call.input({ id: open.id })).confirmed;

    // expect the query of the method's permission, applying to the open task until it is done
    expect({
        query: call.query([open.id]),
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

test("refuse a condition on a call without a target, and a key combination editors do not write", async () => {
    // declare a create with a condition, and read back an action on keys the compiler cannot check
    const owner = { package: notes };
    const open = defineAction(
        { name: "open-note", title: t`Open`, call: { object: note, method: "open" } },
        owner,
    );
    const refused = [
        await refusal(() =>
            defineAction(
                {
                    name: "new-note",
                    title: t`New Note`,
                    call: { object: note, method: "create", when: { status: "open" } },
                },
                owner,
            ),
        ),
        await refusal(() =>
            Action.of("open-note", { ...open.describe(), keybinding: "hyper+o" }, note),
        ),
    ];

    // expect both refused, the condition by its method and the keys by their syntax
    expect(refused).toEqual([
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "action 'new-note' is invalid: it has a condition, but 'note.create' acts on its scope",
        ],
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "key combination 'hyper+o' of action 'open-note' is invalid: editors do not write it",
        ],
    ]);
});

test("list a package's actions on its own objects, refusing a call of a type it does not serve or an action named as a method", async () => {
    // declare actions on tasks, on notes and one named as the tasks' mark method
    const owner = { package: task.package };
    const markDone = defineAction(
        {
            name: "mark-done",
            title: t`Mark Done`,
            call: { object: task, method: "mark", input: { status: "done" } },
        },
        owner,
    );
    const stray = defineAction(
        { name: "finish", title: t`Finish`, call: { object: note, method: "mark" } },
        owner,
    );
    const named = defineAction(
        { name: "mark", title: t`Mark`, call: { object: task, method: "mark" } },
        owner,
    );

    // list each on a package serving the tasks alone
    const handle = definePackage(
        {
            ...testIdentity,
            objects: [task],
            actions: [markDone],
        },
        owner,
    );
    const refused = [
        await refusal(() =>
            definePackage(
                {
                    ...testIdentity,
                    objects: [task],
                    actions: [stray],
                },
                owner,
            ),
        ),
        await refusal(() =>
            definePackage(
                {
                    ...testIdentity,
                    objects: [task],
                    actions: [named],
                },
                owner,
            ),
        ),
    ];

    // expect the action listed, and both others refused by name
    expect({ listed: Object.keys(handle.actions), refused }).toEqual({
        listed: ["mark-done"],
        refused: [
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                `action 'finish' is invalid: '${task.package.name}' lists it on note, which it does not serve`,
            ],
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                "action 'mark' is invalid: it takes the name of a method of task",
            ],
        ],
    });
});

/** Read what an action's call acts on, the input it asks for and whether it removes for good. */
function readCall(action: Action) {
    const call = present(action.call, "the action's call");
    const { properties } = schema
        .looseObject({ properties: schema.jsonObject() })
        .parse(call.parameters);

    return {
        target: call.target,
        asked: Object.keys(properties),
        isDestructive: call.isDestructive,
    };
}

test("offer the declared actions meeting their condition, then each titled method and valid transition", () => {
    // define tickets moving from todo through started to done, with an edit, a delete and a ping
    const ticket = defineObject(
        {
            name: "ticket",
            plural: "tickets",
            title: t`${plural(argument("count"), { one: "Ticket", other: "Tickets" })}`,
            scope: device,
            fields: {
                title: field.string(schema.string()),
                state: field.state({
                    initial: "todo",
                    transitions: {
                        start: { from: ["todo"], to: "started", permission: "update" },
                        finish: { from: ["started"], to: "done", permission: "update" },
                    },
                }),
            },
            access: { permissions: { read: none(), update: none() } },
        },
        (method) => ({
            edit: method.update("update", { fields: ["title"] }),
            delete: method.delete("update"),
            ping: method.mutation({ title: t`Ping`, permission: "read" }),
            sync: method.mutation({ title: t`Sync`, permission: "read", visibility: "package" }),
        }),
    );

    // escalate a ticket still to do through its ping
    const escalate = new Action(notes, {
        name: "escalate",
        title: t`Escalate`,
        call: { object: ticket, method: "ping", when: { state: "todo" } },
    });
    const todo = { id: "ticket-1", title: "Write", state: "todo" };
    const started = { id: "ticket-2", title: "Ship", state: "started" };
    const names = (rows: readonly (typeof todo)[]) =>
        Action.available(ticket, rows, [escalate], { now: 0, timeZone: "UTC" }).map(
            (each) => each.name,
        );

    expect({
        todo: names([todo]),
        started: names([started]),
        both: names([todo, started]),
    }).toEqual({
        todo: ["escalate", "edit", "delete", "start"],
        started: ["edit", "delete", "finish"],
        both: ["edit", "delete"],
    });
});
