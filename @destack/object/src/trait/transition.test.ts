import { principal, through } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import { argument, plural, t } from "@destack/locale";
import { Identifier, present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { ObjectRemote } from "../client/index.ts";
import { ObjectClient } from "../device/index.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/index.ts";
import { device } from "../object/device.ts";
import { testSimulation } from "../test/index.ts";

/** A device's tasks, done or reopened through their state's transitions. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: device,
        storage: "local",
        fields: {
            title: field.string(),
            status: field.state({
                initial: "open",
                final: ["done", "canceled"],
                transitions: {
                    complete: { from: ["open"], to: "done", permission: "update" },
                    cancel: { from: ["open"], to: "canceled", permission: "update" },
                    reopen: { from: ["done"], to: "open", permission: "update" },
                },
            }),
        },
        access: {
            permissions: {
                read: through("device", "use"),
                create: through("device", "use"),
                update: through("device", "use"),
            },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["title"] }),
        update: method.update("update"),
    }),
);

test("move a state an update sets through the transition between its states, with the other fields", async () => {
    // create a task, then set its status and title in one update, and its status back
    const tasks = await openTasks();
    const created = await tasks.mutate(task).create({ title: "Write" }).predicted;
    await tasks.mutate(task).update({ id: created.id, status: "done", title: "Written" }).confirmed;
    const done = await tasks.query.task.findFirst({ where: { id: created.id } });
    await tasks.mutate(task).update({ id: created.id, status: "open" }).confirmed;
    const reopened = await tasks.query.task.findFirst({ where: { id: created.id } });

    expect([done?.status, done?.title, reopened?.status]).toEqual(["done", "Written", "open"]);
});

test("refuse a state an update sets where no transition leads from the current one", async () => {
    // finish a task, then cancel it, which only an open task may be
    const tasks = await openTasks();
    const created = await tasks.mutate(task).create({ title: "Write" }).predicted;
    await tasks.mutate(task).update({ id: created.id, status: "done" }).confirmed;
    const canceled = tasks.mutate(task).update({ id: created.id, status: "canceled" }).confirmed;

    await expect(canceled).rejects.toMatchObject({
        reason: "TRANSITION_BLOCKED",
        message: "cannot move task 'status' from 'done' to 'canceled': no transition leads there",
    });
});

test("undo an update's state change only along a transition back, else leave it undoable by nothing", () => {
    // undo a completion, which reopening reverses, and a cancellation, which nothing reverses
    const { update } = task.methods;
    const scope = present(task.route.field, "the task's scope field");
    const undo = (status: string) =>
        update.inverse?.({
            object: task,
            name: "update",
            input: { [scope]: "device-1", id: "task-1", status },
            before: { status: "open" },
            after: { status },
            current: { status },
        });

    expect([undo("done"), undo("canceled")]).toEqual([
        [
            {
                type: task.typeReference,
                method: "update",
                release: task.package.version,
                edition: 1,
                input: { [scope]: "device-1", id: "task-1", status: "open" },
            },
        ],
        undefined,
    ]);
});

/** Store a device's tasks in memory, as the device. */
async function openTasks(): Promise<ObjectClient<{ task: typeof task }>> {
    const environment = testSimulation().environment("device");
    const database = await connectBunSqlite(":memory:", [], { environment });
    onTestFinished(() => database.close());
    const id = Identifier.create("device", environment);
    const caller = principal.device.reference(Scope.universe.id, id);
    const client = await ObjectClient.open({
        environment,
        database,
        objects: { task },
        scope: caller.id,
        caller,
        device: caller,
        remote: ObjectRemote.local(() => new TypeError("a device's tasks call no server")),
    });
    onTestFinished(() => client.close());

    return client;
}
