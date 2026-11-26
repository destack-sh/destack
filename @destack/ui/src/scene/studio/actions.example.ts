import { t } from "@destack/locale";
import { current, defineAction, selected } from "@destack/object";
import { type JsonValue, PlainDate, schema } from "@destack/schema";
import { handle, useActions } from "@destack/view";
import { commentType, taskPriority, taskStatus, taskType } from "./object.example.ts";
import { idsOf, LABELS, PEOPLE, PROJECTS, type Studio, type Task, TODAY } from "./state.ts";

/** Change the status of tasks. */
export const setStatus = defineAction({
    name: "task.setStatus",
    title: t.context("action")`Change status`,
    category: t.context("title")`Task`,
    keybinding: "s",
    input: { tasks: selected(taskType), status: taskStatus },
});

/** Change the priority of tasks. */
export const setPriority = defineAction({
    name: "task.setPriority",
    title: t.context("action")`Set priority`,
    category: t.context("title")`Task`,
    keybinding: "p",
    input: { tasks: selected(taskType), priority: taskPriority },
});

/** Assign tasks to a person. */
export const assign = defineAction({
    name: "task.assign",
    title: t.context("action")`Assign to`,
    category: t.context("title")`Task`,
    keybinding: "a",
    input: { tasks: selected(taskType), person: schema.string().nullable() },
});

/** Assign tasks to the person at the keys. */
export const assignToMe = defineAction({
    name: "task.assignToMe",
    title: t`Assign to me`,
    category: t.context("title")`Task`,
    keybinding: "i",
    input: { tasks: selected(taskType) },
});

/** Add a label to tasks, or take it off. */
export const toggleLabel = defineAction({
    name: "task.toggleLabel",
    title: t.context("action")`Change labels`,
    category: t.context("title")`Task`,
    keybinding: "l",
    isRepeated: true,
    input: { tasks: selected(taskType), label: schema.string() },
});

/** Add tasks to a project. */
export const setProject = defineAction({
    name: "task.setProject",
    title: t`Add to project`,
    category: t.context("title")`Task`,
    keybinding: "shift+p",
    input: { tasks: selected(taskType), project: schema.string().nullable() },
});

/** Set the day tasks are due. */
export const setDue = defineAction({
    name: "task.setDue",
    title: t`Set due date`,
    category: t.context("title")`Task`,
    keybinding: "shift+d",
    input: { tasks: selected(taskType), due: schema.string().nullable() },
});

/** Create a task, with what the group the person is in has. */
export const createTask = defineAction({
    name: "task.create",
    title: t.context("action")`New task`,
    category: t.context("title")`Task`,
    keybinding: "c",
});

/** Look at a task without leaving the list. */
export const peek = defineAction({
    name: "task.peek",
    title: t.context("action")`Peek`,
    category: t.context("title")`Task`,
    keybinding: "space",
    input: { task: current(taskType) },
});

/** Delete tasks. */
export const deleteTasks = defineAction({
    name: "task.delete",
    title: t.context("action")`Delete`,
    category: t.context("title")`Task`,
    keybinding: { key: "mod+delete", mac: ["mod+backspace"] },
    isDestructive: true,
    input: { tasks: selected(taskType) },
});

/** Show the tasks as rows, or as cards. */
export const toggleLayout = defineAction({
    name: "view.toggleLayout",
    title: t`Show as a board`,
    category: t.context("title")`View`,
    keybinding: "shift+b",
});

/** Go to the tasks people work on. */
export const goToActive = defineAction({
    name: "studio.goToActive",
    title: t`Go to active tasks`,
    category: t.context("title")`Navigation`,
    keybinding: "g a",
});

/** Go to the tasks nobody started. */
export const goToBacklog = defineAction({
    name: "studio.goToBacklog",
    title: t`Go to the backlog`,
    category: t.context("title")`Navigation`,
    keybinding: "g b",
});

/** Go to every task. */
export const goToAll = defineAction({
    name: "studio.goToAll",
    title: t`Go to all tasks`,
    category: t.context("title")`Navigation`,
    keybinding: "g e",
});

/** Choose how the view looks. */
export const showDisplay = defineAction({
    name: "view.display",
    title: t.context("action")`Display options`,
    category: t.context("title")`View`,
    keybinding: "shift+v",
});

/** List the days people set things due on. */
function days() {
    return [
        { value: TODAY, title: "Today" },
        { value: PlainDate.add(TODAY, { days: 1 }), title: "Tomorrow" },
        { value: PlainDate.add(TODAY, { days: 5 }), title: "Next Monday" },
        { value: PlainDate.add(TODAY, { days: 7 }), title: "In one week" },
        { value: PlainDate.add(TODAY, { days: 14 }), title: "In two weeks" },
        { value: null, title: "No due date" },
    ];
}

/** Answer a comment. */
export const answerComment = defineAction({
    name: "comment.answer",
    title: t.context("action")`Reply`,
    category: t.context("title")`Comment`,
    keybinding: "r",
    input: { comment: current(commentType) },
});

/** React to a comment, or take the reaction back. */
export const reactToComment = defineAction({
    name: "comment.react",
    title: t.context("action")`React`,
    category: t.context("title")`Comment`,
    input: { comment: current(commentType), emoji: schema.string() },
});

/** Write a comment anew. */
export const editComment = defineAction({
    name: "comment.edit",
    title: t.context("action")`Edit`,
    category: t.context("title")`Comment`,
    keybinding: "e",
    input: { comment: current(commentType) },
});

/** Copy the words of a comment. */
export const copyComment = defineAction({
    name: "comment.copy",
    title: t.context("action")`Copy text`,
    category: t.context("title")`Comment`,
    input: { comment: current(commentType) },
});

/** Delete a comment and its replies. */
export const deleteComment = defineAction({
    name: "comment.delete",
    title: t.context("action")`Delete`,
    category: t.context("title")`Comment`,
    keybinding: { key: "delete", mac: ["backspace", "delete"] },
    isDestructive: true,
    input: { comment: current(commentType) },
});

/** Handle the actions on the tasks of a studio in an element. */
export function handleTasks(studio: Studio): ((element: Element) => void)[] {
    // read the values the tasks where the person is share
    const actions = useActions();
    const shared = (read: (each: Task) => readonly (string | null)[]) => {
        return (_input: string, element: Element | null): readonly JsonValue[] => {
            const lists = selected(taskType)
                .read(actions.places(element))
                .flatMap(({ id }) => {
                    const found = studio.find(id);

                    return found === undefined ? [] : [read(found)];
                });

            return (lists[0] ?? []).filter((value) => lists.every((each) => each.includes(value)));
        };
    };

    return [
        // set a property
        handle(setStatus, {
            chosen: shared((each) => [each.status]),
            run: ({ values }) => studio.update(idsOf(values.tasks), { status: values.status }),
        }),
        handle(setPriority, {
            chosen: shared((each) => [each.priority]),
            run: ({ values }) => studio.update(idsOf(values.tasks), { priority: values.priority }),
        }),
        handle(setProject, {
            chosen: shared((each) => [each.project]),
            choices: () => PROJECTS,
            run: ({ values }) => studio.update(idsOf(values.tasks), { project: values.project }),
        }),
        handle(setDue, {
            chosen: shared((each) => [each.due]),
            choices: (_input, element) => {
                // suggest the day a task has too
                const [first] = selected(taskType).read(actions.places(element));
                const due = studio.find(first?.id ?? "")?.due ?? null;
                const listed = days();

                return listed.some(({ value }) => value === due)
                    ? listed
                    : [{ value: due, title: String(due) }, ...listed];
            },
            run: ({ values }) =>
                studio.update(idsOf(values.tasks), {
                    due: values.due,
                }),
        }),

        // assign them
        handle(assign, {
            chosen: shared((each) => [each.assignee]),
            choices: () => [
                { value: null, title: "No assignee" },
                ...PEOPLE.map(({ id, name }) => ({ value: id, title: name })),
            ],
            run: ({ values }) =>
                studio.update(idsOf(values.tasks), {
                    assignee: values.person,
                }),
        }),
        handle(assignToMe, {
            run: ({ values }) => studio.update(idsOf(values.tasks), { assignee: "ada" }),
        }),

        // add a label, or take it off where every task has it
        handle(toggleLabel, {
            chosen: shared((each) => each.labels),
            choices: () => LABELS,
            run: ({ values }) => studio.toggleLabel(idsOf(values.tasks), values.label),
        }),

        // delete them
        handle(deleteTasks, { run: ({ values }) => studio.remove(idsOf(values.tasks)) }),
    ];
}
