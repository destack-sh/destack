import { principal, through } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import { argument, plural, t } from "@destack/locale";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { ObjectRemote } from "../client/index.ts";
import { ObjectClient } from "../device/index.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/index.ts";
import { device } from "../object/device.ts";
import { testSimulation } from "../test/index.ts";

/** The permissions of a device's own objects. */
const access = {
    permissions: {
        read: through("device", "use"),
        create: through("device", "use"),
        update: through("device", "use"),
    },
};

/** A device's tasks. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: device,
        storage: "local",
        fields: { title: field.string() },
        access,
    },
    (method) => ({ create: method.create("create") }),
);

/** A device's labels. */
const label = defineObject(
    {
        name: "label",
        plural: "labels",
        title: t`${plural(argument("count"), { one: "Label", other: "Labels" })}`,
        scope: device,
        storage: "local",
        fields: { name: field.string() },
        access,
    },
    (method) => ({ create: method.create("create"), update: method.update("update") }),
);

/** The labels on each task, joining tasks and labels to each other. */
const taskLabel = defineObject(
    {
        name: "task-label",
        plural: "taskLabels",
        title: t`${plural(argument("count"), { one: "Task label", other: "Task labels" })}`,
        scope: device,
        storage: "local",
        isJunction: true,
        fields: { task: field.reference(task), label: field.reference(label) },
        access,
    },
    (method) => ({ create: method.create("create") }),
);

test("relate the two types a junction joins to each other by the other's plural", async () => {
    // label one task urgent and blocked, and another urgent
    const client = await openClient();
    const write = client.mutate(taskLabel);
    const [ship, plan] = [
        await client.mutate(task).create({ title: "Ship" }).predicted,
        await client.mutate(task).create({ title: "Plan" }).predicted,
    ];
    const [urgent, blocked] = [
        await client.mutate(label).create({ name: "urgent" }).predicted,
        await client.mutate(label).create({ name: "blocked" }).predicted,
    ];
    await write.create({ taskId: ship.id, labelId: urgent.id }).predicted;
    await write.create({ taskId: ship.id, labelId: blocked.id }).predicted;
    await write.create({ taskId: plan.id, labelId: urgent.id }).predicted;

    // read each task's labels and each label's tasks through the junction
    const tasks = await client.query.task.findMany({
        with: { labels: { orderBy: { name: "asc" } } },
        orderBy: { title: "asc" },
    });
    const labels = await client.query.label.findMany({
        with: { tasks: { orderBy: { title: "asc" } } },
        orderBy: { name: "asc" },
    });

    expect({
        tasks: tasks.map((row) => [row.title, row.labels.map((each) => each.name)]),
        labels: labels.map((row) => [row.name, row.tasks.map((each) => each.title)]),
    }).toEqual({
        tasks: [
            ["Plan", ["urgent"]],
            ["Ship", ["blocked", "urgent"]],
        ],
        labels: [
            ["blocked", ["Ship"]],
            ["urgent", ["Plan", "Ship"]],
        ],
    });
});

test("follow a task's labels live as a junction row joins one and the label changes", async () => {
    // follow a task's labels before it has any
    const client = await openClient();
    const ship = await client.mutate(task).create({ title: "Ship" }).predicted;
    const urgent = await client.mutate(label).create({ name: "urgent" }).predicted;
    const live = client.query.task.findMany({ with: { labels: true } }).subscribe();
    const stopping = new AbortController();
    onTestFinished(async () => {
        stopping.abort();
        await live.close();
    });
    const values = live.watch(stopping.signal);
    const names = async () => {
        const next = await values.next();

        return next.done === true
            ? []
            : next.value.map((row) => row.labels.map((each) => each.name));
    };

    // read the task bare, then labelled through a new junction row, then with the label renamed
    const bare = await names();
    await client.mutate(taskLabel).create({ taskId: ship.id, labelId: urgent.id }).predicted;
    const labelled = await names();
    await client.mutate(label).update({ id: urgent.id, name: "now" }).predicted;
    const renamed = await names();

    expect([bare, labelled, renamed]).toEqual([[[]], [["urgent"]], [["now"]]]);
});

test("refuse a junction that references other than two types", () => {
    expect(() =>
        defineObject({
            name: "task-owner",
            plural: "taskOwners",
            title: t`${plural(argument("count"), { one: "Task owner", other: "Task owners" })}`,
            scope: device,
            storage: "local",
            isJunction: true,
            fields: { task: field.reference(task) },
            access,
        }),
    ).toThrow(
        new TypeError(
            "object 'task-owner' is a junction, so it references exactly two types, not 1",
        ),
    );
});

/** Store a device's tasks, labels and their junction in memory, as the device. */
async function openClient() {
    const environment = testSimulation().environment("device");
    const database = await connectBunSqlite(":memory:", [], { environment });
    onTestFinished(() => database.close());
    const id = Identifier.create("device", environment);
    const caller = principal.device.reference(Scope.universe.id, id);
    const client = await ObjectClient.open({
        environment,
        database,
        objects: { task, label, taskLabel },
        scope: caller.id,
        caller,
        device: caller,
        remote: ObjectRemote.local(() => new TypeError("a device's own objects call no server")),
    });
    onTestFinished(() => client.close());

    return client;
}
