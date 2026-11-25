import calendarBlank from "@destack/icon/phosphor/calendar-blank";
import starIcon from "@destack/icon/phosphor/star";
import trash from "@destack/icon/phosphor/trash";
import { argument, plural, t } from "@destack/locale";
import { defineAction, field, ObjectError } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { createSignal, handleOnPage, type JSX } from "@destack/view";
import { Button } from "../button/index.ts";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "../dropdown-menu/index.ts";
import { MenuAction } from "../menu/index.ts";
import { Cluster } from "../cluster/index.ts";
import { Stack } from "../stack/index.ts";
import { Toaster } from "../toast/index.ts";
import { ActionButton } from "./action-button.tsx";
import { ActionDialog } from "./action-dialog.tsx";
import { ActionSelect } from "./action-select.tsx";

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

/** Star the task. */
const star = defineAction({ name: "task.star", title: t`Star`, keybinding: "b" });

/** Postpone the task. */
const postpone = defineAction({
    name: "task.postpone",
    title: t`Postpone`,
    summary: t`Postpone by ${plural(argument("days"), { one: "# day", other: "# days" })}`,
    input: { days: field.integer().labeled(t`Days`) },
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

/** Render a task's status and person, each picked beside its button. */
function TaskProperties(): JSX.Element {
    // keep the status and the person
    const [status, setStatus] = createSignal("Doing");
    const [person, setPerson] = createSignal<string | undefined>(undefined);
    handleOnPage(changeStatus, {
        choices: () => STATUSES,
        chosen: () => [status()],
        run: ({ values }) => void setStatus(values.status),
    });
    handleOnPage(assign, {
        choices: () => PEOPLE.map(({ name, icon }) => ({ value: name, title: name, icon })),
        chosen: () => {
            const current = person();

            return current === undefined ? [] : [current];
        },
        run: ({ values }) => void setPerson(values.person),
    });

    return (
        <Cluster>
            <ActionSelect action={changeStatus} />
            <ActionSelect action={assign} variant="outline" placeholder="Assignee" />
            <ActionDialog />
        </Cluster>
    );
}

/** A task's status and person, each shown on the button that picks another. */
export const actionSelectTask = defineExample({
    of: ActionSelect,
    name: "select",
    description: "a task's status and person, each shown on the button that picks another",
    render: () => <TaskProperties />,
});

/** Render a task's actions as buttons, reporting what each did. */
function TaskActions(): JSX.Element {
    // report what each action did
    const [done, setDone] = createSignal("Press a button, or its keys");
    handleOnPage(assign, {
        choices: () => PEOPLE.map(({ name, icon }) => ({ value: name, title: name, icon })),
        run: ({ values }) => void setDone(`Assigned to ${values.person}`),
    });
    handleOnPage(changeStatus, {
        choices: () => STATUSES,
        run: ({ values }) => void setDone(`Moved to ${values.status}`),
    });
    handleOnPage(plan, {
        run: ({ values }) => void setDone(`Planned ${values.title}, ${values.points} points`),
    });
    const [isStarred, setStarred] = createSignal(false);
    handleOnPage(star, { isOn: isStarred, run: () => void setStarred((is) => !is) });
    handleOnPage(postpone, {
        run: ({ values }) => void setDone(`Postponed by ${values.days} days`),
    });
    handleOnPage(publish, {
        run: () => {
            throw ObjectError.of("CALL_BLOCKED", "cannot publish the task: its launch is over");
        },
    });
    handleOnPage(remove, { run: () => void setDone("Deleted") });

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
                <ActionButton action={star} icon={starIcon} size="icon" variant="ghost" />
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={(attributes) => <Button {...attributes} variant="outline" />}
                    >
                        More
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <MenuAction action={star} />
                        <MenuAction action={changeStatus} />
                        <MenuAction action={assign} />
                        <MenuAction action={postpone} given={{ days: 3 }} />
                        <MenuAction action={remove} />
                    </DropdownMenuContent>
                </DropdownMenu>
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
