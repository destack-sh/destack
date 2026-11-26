import { testIdentity } from "../test/index.ts";
import { none, principal } from "@destack/access";
import { now } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { argument, plural, t } from "@destack/locale";
import { PackageVersion, PackageId } from "@destack/package";
import {
    Identifier,
    type JsonObject,
    JsonValue,
    present,
    schema,
    toJsonSchema,
} from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, expectTypeOf, onTestFinished, refusal, test } from "@destack/test";
import { ObjectRemote } from "../client/index.ts";
import { ObjectClient } from "../device/index.ts";
import { testSimulation } from "../test/world.ts";
import { field } from "../field/index.ts";
import { defineObject, device } from "../object/index.ts";
import type { ShortcutText } from "@destack/locale";
import { Action, defineAction } from "./action.ts";
import { ActionCall } from "./call.ts";
import type { ActionKeybindingDefinition } from "./keybinding.ts";
import { Place } from "./place.ts";
import { ActionDescription } from "./description.ts";
import { type ActionContext, Template } from "./template.ts";
import {
    creating,
    current,
    defineVariable,
    me,
    type ObjectInput,
    picked,
    selected,
    space,
    Variable,
    type VariableInput,
    type VariablesOf,
} from "./variable.ts";
import type { CallInput } from "../method/index.ts";
import type { ObjectType } from "../object/index.ts";
import { definePackage } from "./package.ts";
import { archivable, controlled, recoverable } from "../index.ts";

/** The declaring package. */
const notes = PackageVersion.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
});

/** The person the actions run for, at noon on the first of January 2026 in UTC. */
const PERSON: ActionContext & { readonly subject: ObjectReference } = {
    subject: principal.user.reference(
        Scope.universe.id,
        "user-019f7480-0000-7000-8000-000000000002",
    ),
    scope: "device-1",
    moment: { now: 1_767_268_800_000, timeZone: "UTC" },
    places: [],
};

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

test("describe actions by their keys, icons, inputs and template, and read them back from their descriptions", () => {
    // declare an action marking open notes done, one creating notes by a title and one a view handles
    const owner = { package: notes };
    const done = defineAction(
        {
            name: "note.markDone",
            title: t`Mark Done`,
            icon: "check",
            description: t`Close the selected open notes.`,
            summary: t`Mark ${argument("notes")} done`,
            category: t`Note`,
            keywords: t`complete, finish`,
            input: { notes: selected(note, { status: "open" }) },
            call: (variables) => variables.notes.prepare("mark", { status: "done" }),
        },
        owner,
    );
    const create = defineAction(
        {
            name: "note.newNote",
            title: t`New Note`,
            keybinding: "mod+n",
            input: {
                title: field
                    .string(schema.string())
                    .labeled(t`Title`, "text-t")
                    .described(t`The note's heading.`),
            },
            call: ({ title }) => note.prepare("create", { title }),
        },
        owner,
    );
    const pin = defineAction(
        {
            name: "pin",
            title: t`Pin`,
            keybinding: { key: ["mod+p", "plus"], mac: "mod+backspace" },
            focus: "any",
        },
        owner,
    );

    // describe each, then read each back from its description
    const described = [done, create, pin].map((action) => action.describe());
    const reread = described.map((each) => Action.of(each.name, each, () => note).describe());

    // expect the calls, the inputs, the keys and the focus, the same once read back
    const called = { packageId: note.package.id, type: "note" };
    expect({ described, read: reread }).toEqual({
        described: [
            {
                package: notes,
                name: "note.markDone",
                title: t`Mark Done`,
                icon: "check",
                description: t`Close the selected open notes.`,
                summary: t`Mark ${argument("notes")} done`,
                category: t`Note`,
                keywords: t`complete, finish`,
                focus: "page",
                call: {
                    ...called,
                    method: "mark",
                    input: { status: "done" },
                    target: "notes",
                    inputs: [
                        {
                            name: "notes",
                            presence: "required",
                            kind: "object",
                            type: note.typeReference,
                            source: "selected",
                            where: { status: "open" },
                        },
                    ],
                },
            },
            {
                package: notes,
                name: "note.newNote",
                title: t`New Note`,
                keybinding: { key: ["mod+n"], mac: ["mod+n"] },
                focus: "page",
                call: {
                    ...called,
                    method: "create",
                    input: { title: { placeholder: "title" } },
                    inputs: [
                        {
                            name: "title",
                            label: t`Title`,
                            presence: "required",
                            kind: "text",
                            icon: "text-t",
                            description: t`The note's heading.`,
                        },
                    ],
                },
            },
            {
                package: notes,
                name: "pin",
                title: t`Pin`,
                keybinding: { key: ["mod+p", "plus"], mac: ["mod+backspace"] },
                focus: "any",
            },
        ],
        read: described,
    });
});

test("type an action's calls by the methods and inputs they take, its summary by its inputs and its keys by their syntax", () => {
    // expect a variable only where the method's input takes its value
    type Title = VariableInput<CallInput<typeof note, "create">>["title"];
    expectTypeOf<Variable<string>>().toExtend<Title>();
    expectTypeOf<Variable<number>>().not.toExtend<Title>();
    expectTypeOf<Parameters<typeof note.prepare<typeof note, "create">>[1]>().not.toHaveProperty(
        "titel",
    );

    // expect the fields of an input's object, and keys checked against the keybinding syntax
    type Fields = VariablesOf<{ note: ObjectInput<typeof note> }>["note"];
    expectTypeOf<Fields["title"]>().toEqualTypeOf<Variable<string>>();
    expectTypeOf<ShortcutText<"mod+shift+k">>().toEqualTypeOf<"mod+shift+k">();
    expectTypeOf<ShortcutText<"hyper+x">>().toBeNever();
    expectTypeOf<ShortcutText<"mod+">>().toBeNever();
    expectTypeOf<"hyper+x">().not.toExtend<ActionKeybindingDefinition<"hyper+x">>();
});

/** Read what a method's own action acts on, its inputs and whether it removes for good. */
function read(object: ObjectType, method: string) {
    const call = ActionCall.of(object, method);

    return [call.target, call.inputs.map(({ name }) => name), call.isDestructive];
}

test("read the inputs a method's own action asks for, and whether it removes its target", () => {
    // expect each on the selected objects but the create, the deletes without a trash for good
    expect([
        read(note, "mark"),
        read(note, "create"),
        read(note, "delete"),
        read(draft, "delete"),
        read(job, "delete"),
    ]).toEqual([
        ["target", ["target", "status", "priority", "tags"], false],
        [undefined, ["title", "tags"], false],
        ["target", ["target"], true],
        ["target", ["target"], false],
        ["target", ["target"], true],
    ]);
});

test("query the targets an action's call applies to and call it on one", async () => {
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
            name: "task.markDone",
            title: t`Mark Done`,
            input: { tasks: selected(task, { status: "open" }) },
            call: ({ tasks }) => tasks.prepare("mark", { status: "done" }),
        },
        { package: notes },
    );
    const call = present(markDone.call, "the action's call");
    const applies = async (id: string) =>
        (await client.queryType(call.object).findFirst(call.query([id]))) !== undefined;
    const before = [await applies(open.id), await applies(done.id)];
    const target = task.policy.reference(owner.id, open.id);
    await client.perform(markDone, { values: { tasks: target } }).confirmed;

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

test("refuse a call reading what is no input, and a key combination editors do not write", async () => {
    // declare a call reading another action's input, and read back an action on keys the compiler cannot check
    const owner = { package: notes };
    const open = defineAction(
        {
            name: "note.openNote",
            title: t`Open`,
            input: { notes: selected(note) },
            call: (variables) => variables.notes.prepare("open", {}),
        },
        owner,
    );
    const refused = [
        await refusal(() =>
            defineAction(
                {
                    name: "note.newNote",
                    title: t`New Note`,
                    call: () => note.prepare("create", { title: new Variable<string>("title") }),
                },
                owner,
            ),
        ),
        await refusal(() =>
            Action.of(
                "open-note",
                { ...open.describe(), keybinding: { key: ["o"], mac: ["hyper+o"] } },
                () => note,
            ),
        ),
    ];

    // expect both refused, the call by its variable and the keys by their syntax
    expect(refused).toEqual([
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "action 'note.newNote' is invalid: its call reads 'title', which is no input of it",
        ],
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "key combinations 'o', 'hyper+o' of action 'open-note' are invalid: editors do not write them",
        ],
    ]);
});

test("list a package's actions, on another package's objects too, and refuse a name of another type or method", async () => {
    // declare actions on tasks, on notes and one named as the tasks' mark method
    const owner = { package: task.package };
    const markDone = defineAction(
        {
            name: "task.markDone",
            title: t`Mark Done`,
            input: { tasks: selected(task) },
            call: ({ tasks }) => tasks.prepare("mark", { status: "done" }),
        },
        owner,
    );
    const foreign = defineAction(
        {
            name: "note.finish",
            title: t`Finish`,
            input: { notes: selected(note) },
            call: (variables) => variables.notes.prepare("mark", { status: "done" }),
        },
        owner,
    );
    const named = await refusal(() =>
        defineAction(
            {
                name: "task.mark",
                title: t`Mark`,
                call: () => task.prepare("create", { status: "done" }),
            },
            owner,
        ),
    );
    const bare = await refusal(() =>
        defineAction(
            {
                name: "finish",
                title: t`Finish`,
                input: { tasks: selected(task) },
                call: ({ tasks }) => tasks.prepare("mark", { status: "done" }),
            },
            owner,
        ),
    );

    // list both on a package serving the tasks alone
    const handle = definePackage(
        {
            ...testIdentity,
            objects: [task],
            actions: [markDone, foreign],
        },
        owner,
    );

    // expect both listed, and the bad names refused
    expect({ listed: Object.keys(handle.actions), refused: [named, bare] }).toEqual({
        listed: ["task.markDone", "note.finish"],
        refused: [
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                "action 'task.mark' is invalid: it calls 'create', but has the name of method 'mark'",
            ],
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                "action 'finish' is invalid: it calls 'task', so its name starts with 'task.'",
            ],
        ],
    });
});

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
    const escalate = defineAction(
        {
            name: "ticket.escalate",
            title: t`Escalate`,
            input: { tickets: selected(ticket, { state: "todo" }) },
            call: ({ tickets }) => tickets.prepare("ping", {}),
        },
        { package: notes },
    );
    const todo = { id: "ticket-1", title: "Write", state: "todo" };
    const started = { id: "ticket-2", title: "Ship", state: "started" };
    const names = (rows: readonly (typeof todo)[]) =>
        ticket.actions(rows, [escalate], { now: 0, timeZone: "UTC" }).map((each) => each.name);

    expect({
        todo: names([todo]),
        started: names([started]),
        both: names([todo, started]),
    }).toEqual({
        todo: ["ticket.escalate", "ticket.edit", "ticket.delete", "ticket.start"],
        started: ["ticket.edit", "ticket.delete", "ticket.finish"],
        both: ["ticket.edit", "ticket.delete"],
    });
});

/** The projects of a device. */
const project = defineObject(
    {
        name: "project",
        plural: "projects",
        title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
        scope: device,
        fields: { title: field.string(schema.string()) },
        access: { permissions: { update: none() } },
    },
    () => ({}),
);

/** The columns of an issue's assignee. */
const ASSIGNEE = ["assigneePackageId", "assigneeType", "assigneeScope", "assigneeId"] as const;

/** The issues of a device, each in a project. */
const issue = defineObject(
    {
        name: "issue",
        plural: "issues",
        title: t`${plural(argument("count"), { one: "Issue", other: "Issues" })}`,
        scope: device,
        fields: {
            title: field.string(schema.string()).labeled(t`Title`),
            status: field
                .enum(["todo", "doing", "done"])
                .default("todo")
                .labeled(t`Status`),
            assignee: field
                .subject(principal.user)
                .optional()
                .labeled(t`Assignee`),
            project: field
                .reference(project, { delete: "keep" })
                .optional()
                .labeled(t`Project`),
            dueAt: field
                .time()
                .optional()
                .labeled(t`Due`),
        },
        access: { permissions: { create: none(), update: none() } },
    },
    (method) => ({
        create: method.create("create", {
            fields: ["title", "status", ...ASSIGNEE, "projectId", "dueAt"],
        }),
        edit: method.update("update", {
            fields: ["title", "status", ...ASSIGNEE, "projectId", "dueAt"],
        }),
    }),
);

/** Write the calls of an action on values for the person, reading each object's row from some. */
function invoked(
    action: Action,
    values: JsonObject,
    rows: readonly (JsonObject & { readonly id: string })[] = [],
    context: ActionContext = PERSON,
) {
    return present(action.call, "the action's call").invocations(
        { values },
        { context, read: async (_type, { id }) => rows.find((row) => row.id === id) },
    );
}

/** The identifiers of two issues. */
const ISSUES = [
    "issue-019f7480-0000-7000-8000-000000000003",
    "issue-019f7480-0000-7000-8000-000000000004",
] as const;

test("read the person, the time and a field of an input's object where the action runs", async () => {
    // take an issue oneself due in a day, and add one under another in its project
    const owner = { package: notes };
    const take = defineAction(
        {
            name: "issue.take",
            title: t`Assign to me`,
            input: { issues: selected(issue) },
            call: ({ issues }) =>
                issues.prepare("edit", { assignee: me, dueAt: now.plus({ days: 1 }) }),
        },
        owner,
    );
    const add = defineAction(
        {
            name: "issue.addSimilar",
            title: t`New similar issue`,
            input: {
                similar: current(issue),
                title: field.string(schema.string()).labeled(t`Title`),
                due: field
                    .time()
                    .labeled(t`Due`)
                    .optional(),
            },
            call: ({ similar, title, due }) =>
                issue.prepare("create", {
                    title,
                    dueAt: due,
                    projectId: similar.projectId,
                    assignee: similar.assignee,
                }),
        },
        owner,
    );
    const one = issue.policy.reference("device-1", ISSUES[0]);
    const two = issue.policy.reference("device-2", ISSUES[1]);
    const row = {
        id: ISSUES[0],
        projectId: "project-1",
        ...ObjectReference.columns("assignee", PERSON.subject),
    };

    expect({
        take: await invoked(take, { issues: [one, two] }),
        add: await invoked(add, { similar: one, title: "Write" }, [row]),
    }).toEqual({
        // one call for each target, in its own scope
        take: ISSUES.map((id, index) => ({
            scope: `device-${index + 1}`,
            input: {
                id,
                ...ObjectReference.columns("assignee", PERSON.subject),
                dueAt: 1_767_355_200_000,
            },
        })),
        // the optional due date left out
        add: [
            {
                scope: undefined,
                input: {
                    title: "Write",
                    projectId: "project-1",
                    ...ObjectReference.columns("assignee", PERSON.subject),
                },
            },
        ],
    });
});

/** The reviews of a device, each by a person stored by their identifier. */
const review = defineObject(
    {
        name: "review",
        plural: "reviews",
        title: t`${plural(argument("count"), { one: "Review", other: "Reviews" })}`,
        scope: device,
        fields: { reviewer: field.reference(principal.user).optional() },
        access: { permissions: { update: none() } },
    },
    (method) => ({ edit: method.update("update", { fields: ["reviewerId"] }) }),
);

test("read the person's identifier where a field stores one, refusing to while nobody is signed in", async () => {
    // take a review oneself
    const taken = defineAction(
        {
            name: "review.reviewMyself",
            title: t`Review myself`,
            input: { reviews: selected(review) },
            call: ({ reviews }) => reviews.prepare("edit", { reviewerId: me.id }),
        },
        { package: notes },
    );
    const values = { reviews: review.policy.reference("device-1", "review-1") };

    expect(await invoked(taken, values)).toEqual([
        { scope: "device-1", input: { id: "review-1", reviewerId: PERSON.subject.id } },
    ]);
    await expect(invoked(taken, values, [], { ...PERSON, subject: undefined })).rejects.toThrow(
        new TypeError("variable 'me' has no person: nobody is signed in"),
    );
});

test("read what is selected, and what is created, at some places", () => {
    // select issues and projects in a list, inside a page of one issue
    const [one, two] = ["issue-1", "issue-2"].map((id) => issue.reference("device-1", id));
    const atlas = project.reference("device-1", "project-1");
    const first = present(one, "an issue");
    const second = present(two, "an issue");
    const page = Place.current(first).with(creating(issue), { status: "todo", title: "Plan" });
    const list = Place.selected([second, atlas]).with(creating(issue), { status: "doing" });

    expect({
        selection: Place.selection([list, page]),
        issues: [
            selected(issue).read([list, page]),
            selected(issue).read([page]),
            selected(issue).read([]),
        ],
        creating: creating(issue).read([list, page]),
    }).toEqual({
        selection: [second, atlas],
        issues: [[second], [first], []],
        creating: { status: "doing", title: "Plan" },
    });
});

/** Fill an action from places, reading what it still asks as each input's name and kind. */
function filled(action: Action, places: readonly Place[], given: JsonObject = {}) {
    const { values, initial, asked } = present(action.call, "the action's call").resolve(
        { places },
        given,
    );

    return { values, initial, asked: asked.map((each) => [each.name, each.value.kind]) };
}

test("fill an action from its places, the nearest first, and ask for the rest", () => {
    // a project's page with a board column, two selected issues and one alone
    const scope = "device-1";
    const atlas = project.policy.reference(scope, "project-1");
    const [one, two, three] = ["issue-1", "issue-2", "issue-3"].map((id) =>
        issue.policy.reference(scope, id),
    );
    const page = new Place().with(current(project), atlas);
    const column = new Place().with(creating(issue), { status: "doing" });
    const single = new Place().with(current(issue), present(one, "an issue"));
    const several = new Place().with(selected(issue), [
        present(two, "an issue"),
        present(three, "an issue"),
    ]);

    // move issues to a picked project, add one under another, and add one to the page's project
    const owner = { package: notes };
    const move = defineAction(
        {
            name: "issue.move",
            title: t`Move to project…`,
            summary: t`Move ${argument("issues")} to ${argument("project")}`,
            input: { issues: selected(issue), project: picked(project) },
            call: (variables) =>
                variables.issues.prepare("edit", { projectId: variables.project.id }),
        },
        owner,
    );
    const addUnder = defineAction(
        {
            name: "issue.addUnder",
            title: t`New similar issue`,
            input: { similar: current(issue), title: field.string(schema.string()) },
            call: ({ similar, title }) =>
                issue.prepare("create", { title, projectId: similar.projectId }),
        },
        owner,
    );
    const add = defineAction(
        {
            name: "issue.add",
            title: t`New issue`,
            input: {
                project: current(project).optional(),
                status: field.enum(["todo", "doing", "done"]).default("todo"),
                title: field.string(schema.string()),
            },
            call: (variables) =>
                issue.prepare("create", {
                    title: variables.title,
                    status: variables.status,
                    projectId: variables.project.id,
                }),
        },
        owner,
    );

    // fill the issues' own edit, a method's
    const { asked: offered } = ActionCall.of(issue, "edit").resolve({ places: [single] }, {});
    expect({
        moveOne: filled(move, [single, column, page]),
        moveSelection: filled(move, [several, single, page]),
        moveNowhere: filled(move, [page]),
        moveGiven: filled(move, [], { issues: [one ?? null], project: atlas }),
        addUnderOne: filled(addUnder, [single, page]),
        addUnderSelection: filled(addUnder, [several, page]),
        addInColumn: filled(add, [column, page]),
        addNowhere: filled(add, []),
        addGiven: filled(add, [column, page], { title: "Write", status: "done" }),
        editOne: offered.map(({ name }) => name),
    }).toEqual({
        // act on the nearest issues, asking for the project on its own page too
        moveOne: { values: { issues: [one] }, initial: {}, asked: [["project", "object"]] },
        moveSelection: {
            values: { issues: [two, three] },
            initial: {},
            asked: [["project", "object"]],
        },
        moveNowhere: {
            values: {},
            initial: {},
            asked: [
                ["issues", "object"],
                ["project", "object"],
            ],
        },
        moveGiven: { values: { issues: [one], project: atlas }, initial: {}, asked: [] },
        // take the one issue around, none of several
        addUnderOne: { values: { similar: one }, initial: {}, asked: [["title", "text"]] },
        addUnderSelection: {
            values: {},
            initial: {},
            asked: [
                ["similar", "object"],
                ["title", "text"],
            ],
        },
        // start an issue as its column and page do
        addInColumn: {
            values: { project: atlas, status: "doing" },
            initial: {},
            asked: [["title", "text"]],
        },
        addNowhere: { values: {}, initial: {}, asked: [["title", "text"]] },
        addGiven: {
            values: { project: atlas, status: "done", title: "Write" },
            initial: {},
            asked: [],
        },
        // offer every field a person can give of a method's own edit
        editOne: ["title", "status", "assignee", "projectId", "dueAt"],
    });
});

test("label the inputs of a method's own action by the fields they write, with what each takes", () => {
    expect(
        ActionCall.of(issue, "edit").inputs.map((each) => [each.name, each.label, each.value.kind]),
    ).toEqual([
        ["target", undefined, "object"],
        ["title", t`Title`, "text"],
        ["status", t`Status`, "choice"],
        ["assignee", t`Assignee`, "person"],
        ["projectId", t`Project`, "object"],
        ["dueAt", t`Due`, "time"],
        ["tags", undefined, "json"],
    ]);
});

/** The teams of a device, each with a name, as variables are read by. */
const team = defineObject(
    {
        name: "team",
        plural: "teams",
        title: t`${plural(argument("count"), { one: "Team", other: "Teams" })}`,
        scope: device,
        fields: {
            name: field.string(schema.string()),
            status: field.enum(["open", "closed"]).default("open"),
        },
        access: { permissions: { create: none(), update: none() } },
    },
    (method) => ({ create: method.create("create"), edit: method.update("update") }),
);

test("read a field named as a variable's own name, and start a creation as its place does", async () => {
    // add a team named after the current one
    const copy = defineAction(
        {
            name: "team.copy",
            title: t`Duplicate`,
            input: { source: current(team) },
            call: ({ source }) => team.prepare("create", { name: source.name }),
        },
        { package: notes },
    );
    const call = present(copy.call, "the action's call");
    const source = team.policy.reference("device-1", "team-1");
    const place = new Place()
        .with(current(team), source)
        .with(creating(team), { status: "closed" });
    const resolved = call.resolve({ places: [place] }, {});
    const row = { id: "team-1", name: "Atlas" };

    expect({
        resolved,
        invocations: await call.invocations(resolved, {
            context: PERSON,
            read: async () => row,
        }),
    }).toEqual({
        // the column's status kept for the created team, which no input sets
        resolved: { values: { source }, initial: { status: "closed" }, asked: [] },
        invocations: [{ scope: undefined, input: { status: "closed", name: "Atlas" } }],
    });
});

test("describe whether an action removes and what it asks for, and read both back", () => {
    // describe an action a view handles as JSON stores it
    const clear = defineAction(
        {
            name: "clear",
            title: t`Clear`,
            isDestructive: true,
            input: { color: field.enum(["red", "blue"]) },
        },
        { package: notes },
    );
    const stored = ActionDescription.parse(JSON.parse(JSON.stringify(clear.describe())));
    const declared = Action.of("clear", stored, () => team);

    expect([declared.isDestructive, declared.inputs]).toEqual([true, clear.inputs]);
});

test("keep the nulls of a list a call prepares", () => {
    const template = Template.of({ order: ["a", null, new Variable("last")] });

    expect(template.bind({ values: { last: "z" }, rows: new Map(), context: PERSON })).toEqual({
        order: ["a", null, "z"],
    });
});

test("offer only what runs on an object as it is: in use, archived, in the trash, or declared", () => {
    // list the methods of pages that archive and recover
    const page = defineObject(
        {
            name: "page",
            plural: "pages",
            title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
            scope: device,
            fields: { title: field.string() },
            access: { permissions: { write: none() } },
            traits: [
                archivable({ by: "write" }),
                recoverable({ within: { days: 30 }, by: "write" }),
            ],
        },
        (method) => ({ update: method.update("write") }),
    );
    const names = (row: JsonObject) =>
        page.actions([{ id: "page-1", ...row }], [], PERSON.moment).map(({ name }) => name);

    expect({
        used: names({}),
        archived: names({ archivedAt: 1 }),
        deleted: names({ deletedAt: 1 }),
        managed: names({ managerInstallationId: "installation-1" }),
        detached: names({ managerInstallationId: "installation-1", detachedAt: 1 }),
    }).toEqual({
        used: ["page.update", "page.delete", "page.archive"],
        archived: ["page.delete", "page.unarchive"],
        deleted: ["page.restore", "page.purge"],
        managed: ["page.update", "page.delete", "page.archive", "page.detach"],
        detached: ["page.update", "page.delete", "page.archive", "page.attach"],
    });
});

test("offer a method's own action beside one that presets some of it", () => {
    // close teams by a preset, and rename them by one of the fields an edit takes
    const owner = { package: notes };
    const close = defineAction(
        {
            name: "team.close",
            title: t`Close`,
            input: { teams: selected(team) },
            call: ({ teams }) => teams.prepare("edit", { status: "closed" }),
        },
        owner,
    );
    const rename = defineAction(
        {
            name: "team.rename",
            title: t`Rename`,
            input: { teams: selected(team), name: field.string(schema.string()) },
            call: ({ teams, name }) => teams.prepare("edit", { name }),
        },
        owner,
    );
    const names = (declared: readonly Action[]) =>
        team.actions([{ id: "team-1" }], declared, PERSON.moment).map((action) => action.name);

    // expect the edit beside both, and the methods every object takes
    expect([names([close]), names([rename])]).toEqual([
        ["team.close", "team.edit"],
        ["team.rename", "team.edit"],
    ]);
});

test("refuse a reserved input name and several objects for one value, and an input's object its condition leaves out", async () => {
    const owner = { package: notes };
    const refused = [
        await refusal(() =>
            defineAction(
                {
                    name: "team.reserved",
                    title: t`Reserved`,
                    input: { me: field.string(schema.string()) },
                    call: (variables) => team.prepare("create", { name: variables.me }),
                },
                owner,
            ),
        ),
        await refusal(() =>
            defineAction(
                {
                    name: "team.several",
                    title: t`Several`,
                    input: { teams: selected(team) },
                    call: ({ teams }) => team.prepare("create", { name: teams.name }),
                },
                owner,
            ),
        ),
    ];

    // read a closed team where only open ones apply, on an input that is no target
    const copy = defineAction(
        {
            name: "team.copyOpen",
            title: t`Duplicate`,
            input: { source: current(team, { status: "open" }) },
            call: ({ source }) => team.prepare("create", { name: source.name }),
        },
        owner,
    );
    const source = team.policy.reference("device-1", "team-1");
    const row = { id: "team-1", name: "Atlas", status: "closed" };

    expect([...refused, await refusal(() => invoked(copy, { source }, [row]))]).toEqual([
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "action 'team.reserved' is invalid: its input 'me' has a name calls read for themselves",
        ],
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "action 'team.several' is invalid: its input 'teams' takes several objects, but the call acts on others",
        ],
        [
            "PRECONDITION_FAILED",
            "CALL_BLOCKED",
            "cannot call 'team.create': it does not apply to team 'team-1'",
        ],
    ]);
});

test("read what a schema takes, its choices, and whether it must be given", () => {
    // a page's own action over each kind of schema
    const group = defineAction(
        {
            name: "group",
            title: t`Group by…`,
            input: {
                field: schema.enum(["status", "assignee"]),
                limit: schema.number().default(10),
                title: schema.string().exactOptional(),
                isShown: schema.boolean(),
                range: schema.object({ start: schema.number() }),
            },
        },
        { package: notes },
    );

    expect(group.inputs.map(({ name, presence, value }) => [name, presence, value])).toEqual([
        [
            "field",
            "required",
            { kind: "choice", choices: [{ value: "status" }, { value: "assignee" }] },
        ],
        ["limit", "default", { kind: "number" }],
        ["title", "optional", { kind: "text" }],
        ["isShown", "required", { kind: "boolean" }],
        [
            "range",
            "required",
            {
                kind: "json",
                schema: JsonValue.object(toJsonSchema(schema.object({ start: schema.number() }))),
            },
        ],
    ]);
});

test("read a declared variable the nearest place gives, the space, and refuse one nobody gives", async () => {
    // plan the week a calendar shows, named by its space
    const owner = { package: notes };
    const visibleWeek = defineVariable(
        {
            name: "visible-week",
            label: t`Week`,
            value: schema.object({ start: schema.number(), end: schema.number() }),
        },
        owner,
    );
    const plan = defineAction(
        {
            name: "issue.planWeek",
            title: t`Plan this week`,
            input: { week: visibleWeek },
            call: ({ week }) => issue.prepare("create", { title: space, dueAt: week.end }),
        },
        owner,
    );
    const call = present(plan.call, "the action's call");
    const calendar = new Place().with(visibleWeek, { start: 1, end: 2 });
    const page = new Place().with(visibleWeek, { start: 3, end: 4 });
    const resolved = call.resolve({ places: [calendar, page] }, {});

    expect({
        described: call.describe().inputs,
        resolved,
        invocations: await invoked(plan, resolved.values),
        missing: await refusal(() => invoked(plan, {})),
        wrong: await Promise.all(
            [
                { issues: project.policy.reference("device-1", "project-1") },
                { issues: "issue-1" },
            ].map((values) =>
                refusal(() =>
                    ActionCall.of(issue, "edit").invocations(
                        { values: { target: values.issues } },
                        { context: PERSON, read: async () => undefined },
                    ),
                ),
            ),
        ),
    }).toEqual({
        described: [
            {
                name: "week",
                label: t`Week`,
                presence: "required",
                kind: "json",
                schema: JsonValue.object(toJsonSchema(visibleWeek.value)),
                variable: visibleWeek.key,
            },
        ],
        resolved: { values: { week: { start: 1, end: 2 } }, initial: {}, asked: [] },
        invocations: [{ scope: undefined, input: { title: "device-1", dueAt: 2 } }],
        missing: [
            "PRECONDITION_FAILED",
            "CALL_BLOCKED",
            "cannot call 'issue.create': input 'week' is missing",
        ],
        // a project, and a bare identifier, where an issue belongs
        wrong: [
            ["BAD_REQUEST", "INPUT_INVALID", "invalid input: target: it takes issue"],
            ["BAD_REQUEST", "INPUT_INVALID", "invalid input: target: it takes issue"],
        ],
    });
});
