import type { JsonCondition, OrderBy } from "@destack/db";
import { Localization } from "@destack/locale";
import { Choice, type Field } from "@destack/object";
import { type JsonObject, type JsonValue, type PlainDate, present } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { type Accessor, createMemo, createSignal, type Setter } from "@destack/view";
import { PICTURES } from "../../example/picture.ts";
import { matchesFilter } from "../../filter/index.ts";
import { commentType, STUDIO, taskPriority, taskStatus, taskType } from "./object.example.ts";

/** The steps of undo the studio remembers. */
const HISTORY = 100;

/** Today in the studio. */
export const TODAY: PlainDate = "2026-11-25";

/** A person of the studio. */
export interface Person {
    /** The id. */
    readonly id: string;
    /** The name. */
    readonly name: string;
    /** The initials. */
    readonly initials: string;
    /** The picture, when they have one. */
    readonly picture?: string;
}

/** A value of a task's property, with how it looks. */
export interface TaskChoice extends Choice {
    /** The value, null for none. */
    readonly value: string | null;
}

/** Something a person did to a task, or wrote about it. */
export interface TaskEvent {
    /** The id. */
    readonly id: string;
    /** Who did it. */
    readonly person: string;
    /** When, in milliseconds since the epoch. */
    readonly at: number;
    /** The property they changed and the value it took, absent for a comment. */
    readonly change?: { readonly property: string; readonly value: JsonValue };
    /** What they wrote, absent for a change. */
    readonly text?: string;
    /** The comment it answers, absent for none. */
    readonly answers?: string;
    /** Who reacted to it, by emoji, absent for nobody. */
    readonly reactions?: Readonly<Record<string, readonly string[]>>;
}

/** A task of the studio. */
export interface Task {
    /** The id, as the team writes it. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** What it says. */
    readonly text: string;
    /** The status. */
    readonly status: string;
    /** The priority. */
    readonly priority: string;
    /** Who does it, null for nobody. */
    readonly assignee: string | null;
    /** The labels. */
    readonly labels: readonly string[];
    /** The project, null for none. */
    readonly project: string | null;
    /** The day it is due, null for none. */
    readonly due: PlainDate | null;
    /** The task it is under, null for none. */
    readonly parent: string | null;
    /** When it was created, in milliseconds since the epoch. */
    readonly created: number;
    /** What people did to it and wrote, the oldest first. */
    readonly events: readonly TaskEvent[];
}

/** The properties of a task a person sets. */
export type TaskPatch = Partial<
    Pick<
        Task,
        | "title"
        | "text"
        | "status"
        | "priority"
        | "assignee"
        | "labels"
        | "project"
        | "due"
        | "parent"
    >
>;

/** A property tasks are grouped by. */
export type TaskGroup = "status" | "priority" | "assignee" | "project";

/** A property a row of a task shows. */
export type TaskField = "id" | "priority" | "labels" | "project" | "due" | "assignee" | "created";

/** A saved way to look at tasks. */
export interface TaskView {
    /** The id. */
    readonly id: string;
    /** The name. */
    readonly name: string;
    /** Which tasks it shows. */
    readonly where: JsonCondition;
    /** Whether the tasks are rows or cards. */
    readonly layout: "list" | "board";
    /** The property they are grouped by. */
    readonly group: TaskGroup;
    /** The property each group is grouped by in turn, null for none. */
    readonly subgroup: TaskGroup | null;
    /** Their order in a group, the one a person gave them without a field. */
    readonly orderBy: OrderBy;
    /** The properties a row shows. */
    readonly fields: readonly TaskField[];
    /** Whether groups without tasks show. */
    readonly hasEmptyGroups: boolean;
    /** Whether tasks under others show. */
    readonly hasSubtasks: boolean;
    /** The collapsed groups. */
    readonly collapsed: readonly string[];
}

/** A group of tasks that share a property's value, or a group in one. */
export interface TaskSection {
    /** The key, with the group's around it. */
    readonly key: string;
    /** The value. */
    readonly choice: TaskChoice;
    /** What a task in it has. */
    readonly patch: Partial<Pick<Task, TaskGroup>>;
    /** Whether it is a group in another. */
    readonly isInner: boolean;
    /** How many tasks it has, those of the groups in it included. */
    readonly count: number;
    /** The tasks, none for a group of groups. */
    readonly tasks: readonly Task[];
}

/** The people of the studio. */
export const PEOPLE: readonly Person[] = [
    { id: "ada", name: "Ada Lovelace", initials: "AL", picture: PICTURES.ada },
    { id: "grace", name: "Grace Hopper", initials: "GH", picture: PICTURES.grace },
    { id: "jonas", name: "Jonas Abiola", initials: "JA" },
    { id: "layla", name: "ليلى حداد", initials: "لح" },
    { id: "max", name: "Maximiliane von Hohenstein-Wartenberg", initials: "MH" },
];

/** The statuses of a task, in the order work moves through them. */
export const STATUSES: readonly TaskChoice[] = choicesOfField(taskStatus);

/** The priorities of a task, the most pressing first. */
export const PRIORITIES: readonly TaskChoice[] = choicesOfField(taskPriority);

/** The labels of a task. */
export const LABELS: readonly TaskChoice[] = [
    { value: "site", title: "Site", color: "green" },
    { value: "story", title: "Story", color: "blue" },
    { value: "plan", title: "Plan", color: "purple" },
    { value: "bug", title: "Bug", color: "red" },
    { value: "print", title: "Print", color: "orange" },
];

/** The projects of the studio. */
export const PROJECTS: readonly TaskChoice[] = [
    { value: "launch", title: "Launch", icon: { icon: "rocket-launch" }, color: "orange" },
    { value: "brand", title: "Brand book", icon: { icon: "book-open" }, color: "purple" },
    { value: "shop", title: "Shop", icon: { icon: "storefront" }, color: "green" },
];

/** The choice of nobody and of no project. */
const NONE: Readonly<Record<"assignee" | "project", TaskChoice>> = {
    assignee: {
        value: null,
        title: "No assignee",
        icon: { icon: "user-circle-dashed" },
        color: "gray",
    },
    project: { value: null, title: "No project", icon: { icon: "cube" }, color: "gray" },
};

/** The fields a view's rows show until a person picks others. */
const FIELDS: readonly TaskField[] = ["id", "priority", "labels", "due", "assignee"];

/** The views the studio starts with. */
const VIEWS: readonly TaskView[] = [
    viewOf("active", "Active", { status: { in: ["review", "doing", "todo"] } }),
    viewOf("backlog", "Backlog", { status: "backlog" }),
    viewOf("all", "All tasks", {}),
];

/** Read the choices of a field as English writes them, the language of the scene. */
function choicesOfField(own: Field): readonly TaskChoice[] {
    const localization = Localization.of("en", []);

    // read each value as text, as the studio's enums and states store it
    return (own.choices ?? []).map((choice) => ({
        ...Choice.of(choice, localization),
        value: String(choice.value),
    }));
}

/** Reference a task, as actions on tasks take it. */
export function referenceOf(id: string): ObjectReference {
    return taskType.reference(STUDIO, id);
}

/** Reference a comment, as actions on comments take it. */
export function referenceOfComment(id: string): ObjectReference {
    return commentType.reference(STUDIO, id);
}

/** List the ids of tasks actions take: the selected, or one. */
export function idsOf(references: ObjectReference | readonly ObjectReference[]): readonly string[] {
    return [references].flat().map(({ id }) => id);
}

/** List the choices of a property tasks are grouped by, in the order their groups stand. */
export function choicesOf(group: TaskGroup): readonly TaskChoice[] {
    if (group === "status") {
        return STATUSES;
    } else if (group === "priority") {
        return PRIORITIES;
    } else if (group === "project") {
        return [...PROJECTS, NONE.project];
    }

    return [...PEOPLE.map(({ id, name }) => ({ value: id, title: name })), NONE.assignee];
}

/** Read the choice of a value, refusing a value of none. */
export function choiceOf(choices: readonly TaskChoice[], value: string | null): TaskChoice {
    return present(
        choices.find((choice) => choice.value === value),
        "choice",
    );
}

/** Read a person, none for nobody. */
export function personOf(id: string | null): Person | undefined {
    return PEOPLE.find((person) => person.id === id);
}

/** Report whether a task's day passed before it was done. */
export function isLate({ due, status }: Task): boolean {
    return due !== null && due < TODAY && status !== "done" && status !== "canceled";
}

/** The studio's tasks and views, with undo, as a server would have them. */
export class Studio {
    /** The tasks, in the order a person placed them. */
    readonly tasks: Accessor<readonly Task[]>;
    /** The tasks by id. */
    readonly #byId: Accessor<ReadonlyMap<string, Task>>;
    /** The views. */
    readonly views: Accessor<readonly TaskView[]>;
    /** The tasks before each change, the newest last. */
    readonly #undone: Accessor<readonly (readonly Task[])[]>;
    /** The tasks after each undone change, the newest last. */
    readonly #redone: Accessor<readonly (readonly Task[])[]>;
    /** Replace the tasks. */
    readonly #setTasks: Setter<readonly Task[]>;
    /** Replace the views. */
    readonly #setViews: Setter<readonly TaskView[]>;
    /** Replace the steps to undo. */
    readonly #setUndone: Setter<readonly (readonly Task[])[]>;
    /** Replace the steps to redo. */
    readonly #setRedone: Setter<readonly (readonly Task[])[]>;
    /** Read the time now, in milliseconds since the epoch. */
    readonly #now: () => number;
    /** The number of the next task. */
    #next: number;

    /** Open a studio with its tasks. */
    constructor(tasks: readonly Task[], now: () => number) {
        // follow the tasks, the views and the steps to undo
        const [current, setCurrent] = createSignal(tasks, { ownedWrite: true });
        const [views, setViews] = createSignal(VIEWS, { ownedWrite: true });
        const [undone, setUndone] = createSignal<readonly (readonly Task[])[]>([], {
            ownedWrite: true,
        });
        const [redone, setRedone] = createSignal<readonly (readonly Task[])[]>([], {
            ownedWrite: true,
        });
        this.tasks = current;
        this.views = views;
        this.#undone = undone;
        this.#redone = redone;
        this.#setTasks = setCurrent;
        this.#setViews = setViews;
        this.#setUndone = setUndone;
        this.#setRedone = setRedone;
        this.#now = now;

        // find tasks by their ids, and number the next after the last
        this.#byId = createMemo(() => new Map(current().map((task) => [task.id, task])));
        this.#next = tasks.length + 1;
    }

    /** Read a task, undefined for an id of none. */
    find(id: string): Task | undefined {
        return this.#byId().get(id);
    }

    /** Read a view, refusing an id of none. */
    view(id: string): TaskView {
        return present(
            this.views().find((view) => view.id === id),
            "view",
        );
    }

    /** Change how a view looks at its tasks. */
    arrange(id: string, patch: Partial<TaskView>): void {
        this.#setViews((views) =>
            views.map((view) => (view.id === id ? { ...view, ...patch } : view)),
        );
    }

    /** Create a task, at the start of the others, and return its id. */
    create(patch: TaskPatch & { readonly title: string }, person = "ada"): string {
        // number it and put it first
        const at = this.#now();
        const id = `HAR-${String(this.#next)}`;
        this.#next += 1;
        const task = taskOf(id, patch, person, at);
        this.#change((tasks) => [task, ...tasks]);

        return id;
    }

    /** Set properties of tasks. */
    update(ids: readonly string[], patch: TaskPatch, person = "ada"): void {
        // write down each property a task did not have
        const at = this.#now();
        const changed = new Set(ids);
        const eventsOf = (task: Task): readonly TaskEvent[] =>
            Object.entries(patch).flatMap(([property, value], index) => {
                const isSame = Object.entries(task).some(
                    ([name, own]) => name === property && own === value,
                );
                const isWritten = property === "title" || property === "text";

                return isSame || isWritten
                    ? []
                    : [
                          {
                              id: `${task.id} ${String(task.events.length + index)}`,
                              person,
                              at,
                              change: { property, value },
                          },
                      ];
            });
        this.#change((tasks) =>
            tasks.map((task) =>
                changed.has(task.id)
                    ? { ...task, ...patch, events: [...task.events, ...eventsOf(task)] }
                    : task,
            ),
        );
    }

    /** Add a label to tasks, or take it off where every one has it. */
    toggleLabel(ids: readonly string[], label: string): void {
        const isOn = ids.every((id) => (this.find(id)?.labels ?? []).includes(label));
        const changed = new Set(ids);
        this.#change((tasks) =>
            tasks.map((each) => {
                const rest = each.labels.filter((own) => own !== label);

                return changed.has(each.id)
                    ? { ...each, labels: isOn ? rest : [...rest, label] }
                    : each;
            }),
        );
    }

    /** Place a task before another, at the end without one, with the properties it takes there. */
    move(id: string, patch: TaskPatch, next: string | undefined): void {
        this.#change((tasks) => {
            // take it out, then put it before its neighbour
            const task = present(this.find(id), "task");
            const others = tasks.filter((each) => each.id !== id);
            const at = next === undefined ? -1 : others.findIndex((each) => each.id === next);

            return others.toSpliced(at === -1 ? others.length : at, 0, { ...task, ...patch });
        });
    }

    /** Remove tasks, and those under them. */
    remove(ids: readonly string[]): void {
        const removed = new Set(ids);
        this.#change((tasks) =>
            tasks.filter(
                (task) =>
                    !removed.has(task.id) && (task.parent === null || !removed.has(task.parent)),
            ),
        );
    }

    /** Write a comment on a task. */
    comment(id: string, text: string, answers?: string, person = "ada"): void {
        // number it after what happened before
        const at = this.#now();
        const count = this.find(id)?.events.length ?? 0;
        const event = {
            id: `${id} ${String(count)}`,
            person,
            at,
            text,
            ...(answers === undefined ? {} : { answers }),
        };
        this.#change((tasks) =>
            tasks.map((task) =>
                task.id === id ? { ...task, events: [...task.events, event] } : task,
            ),
        );
    }

    /** Read a comment, undefined for an id of none. */
    findComment(id: string): TaskEvent | undefined {
        return this.tasks()
            .flatMap(({ events }) => events)
            .find((event) => event.id === id && event.text !== undefined);
    }

    /** Write a comment anew. */
    editComment(id: string, written: string): void {
        this.#changeEvents((events) =>
            events.map((event) => (event.id === id ? { ...event, text: written } : event)),
        );
    }

    /** Remove a comment and what answers it. */
    removeComment(id: string): void {
        this.#changeEvents((events) =>
            events.filter((event) => event.id !== id && event.answers !== id),
        );
    }

    /** Add a person's reaction to a comment, or take it back. */
    react(id: string, emoji: string, person = "ada"): void {
        this.#changeEvents((events) =>
            events.map((event) => {
                // leave the other comments alone
                if (event.id !== id) {
                    return event;
                }
                const people = event.reactions?.[emoji] ?? [];
                const next = people.includes(person)
                    ? people.filter((each) => each !== person)
                    : [...people, person];

                return { ...event, reactions: { ...event.reactions, [emoji]: next } };
            }),
        );
    }

    /** Change what happened to tasks, leaving those it changes nothing of alone. */
    #changeEvents(change: (events: readonly TaskEvent[]) => readonly TaskEvent[]): void {
        this.#change((tasks) => tasks.map((task) => ({ ...task, events: change(task.events) })));
    }

    /** Report whether a change is left to undo. */
    canUndo(): boolean {
        return this.#undone().length > 0;
    }

    /** Report whether an undone change is left to redo. */
    canRedo(): boolean {
        return this.#redone().length > 0;
    }

    /** Undo the newest change. */
    undo(): void {
        const before = this.#undone().at(-1);
        if (before !== undefined) {
            const now = this.tasks();
            this.#setRedone((steps) => [...steps, now]);
            this.#setUndone((steps) => steps.slice(0, -1));
            this.#setTasks(before);
        }
    }

    /** Redo the newest undone change. */
    redo(): void {
        const after = this.#redone().at(-1);
        if (after !== undefined) {
            const now = this.tasks();
            this.#setUndone((steps) => [...steps, now]);
            this.#setRedone((steps) => steps.slice(0, -1));
            this.#setTasks(after);
        }
    }

    /** List a view's tasks by group, each group in the view's order, subtasks under their tasks. */
    sections(view: TaskView): readonly TaskSection[] {
        // take the tasks the view shows
        const moment = { now: this.#now(), timeZone: "Europe/Lisbon" };
        const shown = this.tasks().filter(
            (task) =>
                (view.hasSubtasks || task.parent === null) &&
                matchesFilter(view.where, rowOf(task), moment),
        );

        // group them by the property, and each group by the other
        const { subgroup } = view;
        const outer = grouped(view, view.group, shown, "", {});

        return subgroup === null
            ? outer
            : outer.flatMap((section) => [
                  { ...section, tasks: [] },
                  ...grouped(view, subgroup, section.tasks, section.key, section.patch),
              ]);
    }

    /** Change the tasks, which a person can undo. */
    #change(run: (tasks: readonly Task[]) => readonly Task[]): void {
        // remember the tasks before it, and drop what was undone
        const before = this.tasks();
        this.#setUndone((steps) => [...steps, before].slice(-HISTORY));
        this.#setRedone([]);
        this.#setTasks(run);
    }
}

/** Build a view of a name and a condition, its tasks by status. */
function viewOf(id: string, name: string, where: JsonCondition): TaskView {
    return {
        id,
        name,
        where,
        layout: "list",
        group: "status",
        subgroup: null,
        orderBy: {},
        fields: FIELDS,
        hasEmptyGroups: false,
        hasSubtasks: true,
        collapsed: [],
    };
}

/** Open a task with what it starts with, the rest as new tasks have it. */
export function taskOf(
    id: string,
    patch: TaskPatch & { readonly title: string },
    person: string,
    at: number,
): Task {
    return {
        text: "",
        status: "todo",
        priority: "none",
        assignee: null,
        labels: [],
        project: null,
        due: null,
        parent: null,
        ...patch,
        id,
        created: at,
        events: [{ id: `${id} created`, person, at, change: { property: "created", value: "" } }],
    };
}

/** Group tasks by a property, in the order of its values, each group in the view's order. */
function grouped(
    view: TaskView,
    group: TaskGroup,
    tasks: readonly Task[],
    parent: string,
    patch: Partial<Pick<Task, TaskGroup>>,
): readonly TaskSection[] {
    const groups = Map.groupBy(tasks, (each) => each[group]);

    return choicesOf(group).flatMap((choice) => {
        // leave out the empty groups a view hides
        const own = groups.get(choice.value) ?? [];
        const key = `${parent}/${String(choice.value)}`;

        return own.length === 0 && (!view.hasEmptyGroups || parent !== "")
            ? []
            : [
                  {
                      key,
                      choice,
                      patch: { ...patch, [group]: choice.value },
                      isInner: parent !== "",
                      count: own.length,
                      tasks: nest(sorted(own, view.orderBy)),
                  },
              ];
    });
}

/** Read a task as the row a query would. */
function rowOf({ title, status, priority, assignee, project, due, created }: Task): JsonObject {
    return { title, status, priority, assignee, project, due, created };
}

/** Sort tasks by the fields of an order, those without a value last. */
function sorted(tasks: readonly Task[], orderBy: OrderBy): readonly Task[] {
    // leave the order a person gave them
    const keys = Object.entries(orderBy);
    if (keys.length === 0) {
        return tasks;
    }

    // rank priorities as they are listed, and compare anything else as text
    const ranks = new Map(PRIORITIES.map((choice, index) => [choice.value, String(index)]));
    const read = (task: Task, field: string): string => {
        // take the place of a priority, and a late day for none
        if (field === "priority") {
            return ranks.get(task.priority) ?? "";
        } else if (field === "due") {
            return task.due ?? "9999";
        } else if (field === "created") {
            return String(task.created);
        }

        return task.title;
    };

    return tasks.toSorted((one, other) => {
        // take the first field they differ in
        const steps = keys.map(([field, direction]) => {
            const step = read(one, field).localeCompare(read(other, field));

            return direction === "desc" ? -step : step;
        });

        return steps.find((step) => step !== 0) ?? 0;
    });
}

/** Put each task after the one it is under, where both are in the group. */
function nest(tasks: readonly Task[]): readonly Task[] {
    const ids = new Set(tasks.map((task) => task.id));
    const under = Map.groupBy(tasks, ({ parent }) =>
        parent !== null && ids.has(parent) ? parent : null,
    );

    return (under.get(null) ?? []).flatMap((task) => [task, ...(under.get(task.id) ?? [])]);
}
