import calendarBlank from "@destack/icon/phosphor/calendar-blank";
import trash from "@destack/icon/phosphor/trash";
import { t } from "@destack/locale";
import { defineAction, field, ObjectError } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { createSignal, handleAction, type JSX } from "@destack/view";
import { Cluster } from "../cluster/index.ts";
import { Stack } from "../stack/index.ts";
import { Toaster } from "../toast/index.ts";
import { ActionButton } from "./action-button.tsx";
import { ActionDialog } from "./action-dialog.tsx";

/** The people a task is assigned to. */
const PEOPLE = [
    { name: "Ada Lovelace", icon: { emoji: "🧮" } },
    { name: "Grace Hopper", icon: { emoji: "⚓" } },
    { name: "Radia Perlman", icon: { emoji: "🌳" } },
];

/** The statuses of a task, each with its icon and color. */
const STATUSES = [
    { value: "Backlog", title: "Backlog", icon: { icon: "circle-dashed" }, color: "gray" },
    { value: "Todo", title: "Todo", icon: { icon: "circle" }, color: "gray" },
    { value: "Doing", title: "Doing", icon: { icon: "circle-half" }, color: "yellow" },
    { value: "Done", title: "Done", icon: { icon: "check-circle" }, color: "green" },
] as const;

/** Change the task's status. */
const changeStatus = defineAction({
    name: "change-status",
    title: t`Change status`,
    keybinding: "s",
    input: { status: field.enum(["Backlog", "Todo", "Doing", "Done"]).labeled(t`Status`) },
});

/** Assign the task to a person. */
const assign = defineAction({
    name: "assign",
    title: t`Assign`,
    keybinding: "a",
    input: { person: field.string(schema.string()).labeled(t`Person`) },
});

/** Plan the task. */
const plan = defineAction({
    name: "plan",
    title: t`Plan`,
    keybinding: "p",
    input: {
        title: field.string(schema.string()).labeled(t`Title`),
        status: field.enum(["Backlog", "Todo", "Doing"]).labeled(t`Status`),
        points: field.integer().labeled(t`Points`),
        due: field.time().labeled(t`Due`),
        isUrgent: field.boolean().labeled(t`Urgent`),
    },
});

/** Publish the task, which its space refuses. */
const publish = defineAction({ name: "publish", title: t`Publish` });

/** Remove the task. */
const remove = defineAction({
    name: "remove",
    title: t`Delete`,
    keybinding: "delete",
    isDestructive: true,
});

/** Render a task's actions as buttons, reporting what each did. */
function TaskActions(): JSX.Element {
    // report what each action did
    const [done, setDone] = createSignal("Press a button, or its keys");
    handleAction(assign, {
        choices: () => PEOPLE.map(({ name, icon }) => ({ value: name, title: name, icon })),
        run: ({ values }) => void setDone(`Assigned to ${values.person}`),
    });
    handleAction(changeStatus, {
        choices: () => STATUSES,
        run: ({ values }) => void setDone(`Moved to ${values.status}`),
    });
    handleAction(plan, {
        run: ({ values }) => void setDone(`Planned ${values.title}, ${values.points} points`),
    });
    handleAction(publish, {
        run: () => {
            throw ObjectError.of("CALL_BLOCKED", "cannot publish the task: its launch is over");
        },
    });
    handleAction(remove, { run: () => void setDone("Deleted") });

    return (
        <Stack>
            <Cluster>
                <ActionButton action={assign} variant="outline" />
                <ActionButton action={changeStatus} variant="outline" />
                <ActionButton action={plan} variant="outline" icon={calendarBlank} />
                <ActionButton action={assign} given={{ person: "Ada Lovelace" }} variant="outline">
                    Assign to Ada
                </ActionButton>
                <ActionButton action={publish} variant="outline" />
                <ActionButton action={remove} icon={trash} size="icon" variant="ghost" />
            </Cluster>
            <output>{done()}</output>
            <ActionDialog />
        </Stack>
    );
}

/** A task's actions as buttons with their keys, asking for a person or a plan. */
export const actionButtonTask = defineExample({
    of: ActionButton,
    name: "task",
    description: "a task's actions as buttons with their keys, asking for a person or a plan",
    render: () => (
        <Toaster>
            <TaskActions />
        </Toaster>
    ),
});
