import { nth } from "../collection/index.ts";
import * as schema from "../validate/index.ts";
import { WorkCalendar, WorkWeek } from "./calendar.ts";

/** Which edges a link joins, after MS Project's four dependency types. */
export const ScheduleLinkType = schema.enum([
    "finish-to-start",
    "start-to-start",
    "finish-to-finish",
    "start-to-finish",
]);

/** Which edges a link joins. */
export type ScheduleLinkType = schema.Infer<typeof ScheduleLinkType>;

/** The shape of a link. */
const scheduleLinkSchema = schema.object({
    /** The key of the task the link starts from. */
    from: schema.string(),
    /** The key of the task that waits on it. */
    to: schema.string(),
    /** Which edges it joins. */
    type: ScheduleLinkType,
    /** The elapsed milliseconds the waiting edge follows the other by, a lead when negative. */
    lag: schema.number().int(),
});

/** A dependency between two tasks: the later one waits on an edge of the earlier one. */
export type ScheduleLink = schema.Infer<typeof scheduleLinkSchema>;

/** An edge of a task: its start or its end. */
export type ScheduleEdge = "start" | "end";

/** The edges each type of link joins: the earlier task's, then the waiting task's. */
const LINK_EDGES: Readonly<
    Record<ScheduleLinkType, { readonly from: ScheduleEdge; readonly to: ScheduleEdge }>
> = {
    "finish-to-start": { from: "end", to: "start" },
    "start-to-start": { from: "start", to: "start" },
    "finish-to-finish": { from: "end", to: "end" },
    "start-to-finish": { from: "start", to: "end" },
};

/** A dependency between two tasks, after MS Project's links: the later one waits on the earlier. */
export const ScheduleLink = Object.assign(scheduleLinkSchema, {
    /** Read the edges a type of link joins: the earlier task's, then the waiting task's. */
    edges(type: ScheduleLinkType): { readonly from: ScheduleEdge; readonly to: ScheduleEdge } {
        return LINK_EDGES[type];
    },

    /** Read the type of link that joins an earlier task's edge to a waiting task's edge. */
    joining(from: ScheduleEdge, to: ScheduleEdge): ScheduleLinkType {
        const type = ScheduleLinkType.options.find(
            (each) => LINK_EDGES[each].from === from && LINK_EDGES[each].to === to,
        );
        if (type === undefined) {
            throw new RangeError(`schedule link joining '${from}' to '${to}' has no type`);
        }

        return type;
    },

    /** Report whether two links join the same two tasks, which a schedule links once at most. */
    equals(left: ScheduleLink, right: ScheduleLink): boolean {
        return left.from === right.from && left.to === right.to;
    },
});

/** How a constraint bounds a task's dates, after MS Project's constraint types. */
export const ScheduleConstraintType = schema.enum([
    "start-no-earlier-than",
    "start-no-later-than",
    "finish-no-earlier-than",
    "finish-no-later-than",
    "must-start-on",
    "must-finish-on",
]);

/** How a constraint bounds a task's dates. */
export type ScheduleConstraintType = schema.Infer<typeof ScheduleConstraintType>;

/** A bound on a task's dates: its type and the instant it names. */
export interface ScheduleConstraint {
    /** How it bounds the task. */
    readonly type: ScheduleConstraintType;
    /** The instant, in epoch milliseconds. */
    readonly at: number;
}

/** A task a schedule orders: a key and a start and end in epoch milliseconds. */
export interface ScheduleTask {
    /** The key the links use. */
    readonly key: string;
    /** The start, in epoch milliseconds. */
    readonly start: number;
    /** The end, in epoch milliseconds. */
    readonly end: number;
    /** The bound on its dates, none when it runs as soon as its links allow. */
    readonly constraint?: ScheduleConstraint;
}

/** How a move places the tasks it changes: pushing them later only, or as soon as links allow. */
export type ScheduleMode = "push" | "auto";

/** A task in a schedule's order with its links and its latest start. */
interface ScheduleNode<Task extends ScheduleTask> {
    /** The task. */
    readonly task: Task;
    /** The indexes of the tasks it waits on, each with the link. */
    readonly incoming: { readonly index: number; readonly link: ScheduleLink }[];
    /** The indexes of the tasks that wait on it, each with the link. */
    readonly outgoing: { readonly index: number; readonly link: ScheduleLink }[];
    /** The latest start that keeps the schedule's end. */
    latest: number;
}

/** Tasks ordered by their links, after the critical path method: slack and pushed moves. */
export class Schedule<Task extends ScheduleTask> {
    /** The tasks, each after the ones it waits on. */
    readonly tasks: readonly Task[];
    /** The links, each between two of the tasks. */
    readonly links: readonly ScheduleLink[];
    /** The tasks in order with their links and latest starts. */
    readonly #nodes: readonly ScheduleNode<Task>[];
    /** The index of each task in the order, by its key. */
    readonly #indexes: ReadonlyMap<string, number>;
    /** The calendar whose working time lags and lengths count. */
    readonly #calendar: WorkCalendar;

    /** Order tasks by their links in a calendar's time, refusing repeated keys and cycles. */
    constructor(
        tasks: readonly Task[],
        links: readonly ScheduleLink[],
        options: { readonly calendar?: WorkCalendar } = {},
    ) {
        // order the tasks, then index them and their links in that order
        this.tasks = ordered(tasks, links);
        this.links = links;
        this.#calendar = options.calendar ?? new WorkCalendar("UTC", WorkWeek.of([]));
        this.#indexes = new Map(this.tasks.map((task, index) => [task.key, index]));
        this.#nodes = this.tasks.map((task) => ({ task, incoming: [], outgoing: [], latest: 0 }));
        for (const link of links) {
            const from = this.#index(link.from);
            const to = this.#index(link.to);
            nth(this.#nodes, to).incoming.push({ index: from, link });
            nth(this.#nodes, from).outgoing.push({ index: to, link });
        }

        // walk the order backward from the schedule's end for each task's latest start
        const end = Math.max(...this.tasks.map((task) => task.end));
        for (const node of this.#nodes.toReversed()) {
            node.latest = latestOf(node, end, this.#nodes, this.#calendar);
        }
    }

    /** Report whether the schedule has a task. */
    has(key: string): boolean {
        return this.#indexes.has(key);
    }

    /** Read a task by its key, refusing a missing one. */
    task(key: string): Task {
        return nth(this.tasks, this.#index(key));
    }

    /** Read the time a task can slip without moving the schedule's end, negative past a bound. */
    slack(key: string): number {
        const node = nth(this.#nodes, this.#index(key));

        return this.#calendar.between(node.task.start, node.latest);
    }

    /** Read the time a task can slip without moving any task that waits on it. */
    freeSlack(key: string): number {
        // take the latest start each link out of the task allows, at the waiting task's start
        const node = nth(this.#nodes, this.#index(key));
        if (node.outgoing.length === 0) {
            return this.slack(key);
        }
        const length = this.#calendar.between(node.task.start, node.task.end);
        const latest = Math.min(
            ...node.outgoing.map(({ index, link }) => {
                const to = nth(this.#nodes, index).task;

                return latestStart(
                    link,
                    to.start,
                    this.#calendar.between(to.start, to.end),
                    length,
                    this.#calendar,
                );
            }),
        );

        return this.#calendar.between(node.task.start, latest);
    }

    /** Report whether a task is on the critical path: any slip moves the schedule's end. */
    isCritical(key: string): boolean {
        return this.slack(key) <= 0;
    }

    /** Move tasks to new dates, then push the tasks after them later, or reschedule them all. */
    move(tasks: readonly Task[], options: { readonly mode?: ScheduleMode } = {}): Task[] {
        // take the tasks at their new dates by index, refusing ones the schedule lacks
        const mode = options.mode ?? "push";
        const moved = new Map<number, Task>();
        for (const task of tasks) {
            moved.set(this.#index(task.key), task);
        }
        const given = new Set(moved.keys());
        const current = (index: number): Task => moved.get(index) ?? nth(this.tasks, index);

        // place each later task at the earliest start its links and bound allow, keeping its length
        const first = Math.min(...given);
        for (const [index, node] of this.#nodes.entries()) {
            const task = current(index);
            const earliest = earliestOf(node, task, current, mode, this.#calendar);
            const isPlaced =
                mode === "push"
                    ? index > first && earliest > task.start
                    : !given.has(index) && earliest !== task.start;
            if (isPlaced) {
                const length = this.#calendar.between(task.start, task.end);
                moved.set(index, {
                    ...task,
                    start: earliest,
                    end: this.#calendar.add(earliest, length),
                });
            }
        }

        return [...moved.values()];
    }

    /** Report whether a link may join: both tasks present, unlinked, closing no cycle. */
    allows(link: ScheduleLink): boolean {
        // refuse a missing task, a self link, and a second link between the same two tasks
        const from = this.#indexes.get(link.from);
        const to = this.#indexes.get(link.to);
        if (from === undefined || to === undefined || from === to) {
            return false;
        }
        const isLinked = this.links.some(
            (each) => ScheduleLink.equals(each, link) || ScheduleLink.equals(each, reversed(link)),
        );

        // a link forward in the order keeps it, one backward closes a cycle once its end reaches back
        return !isLinked && (from < to || !this.#reaches(to, from));
    }

    /** Read the index of a task by its key, refusing a missing one. */
    #index(key: string): number {
        const index = this.#indexes.get(key);
        if (index === undefined) {
            throw new RangeError(`schedule task '${key}' not found`);
        }

        return index;
    }

    /** Report whether the links lead from one task to another, up to the target in order. */
    #reaches(from: number, target: number): boolean {
        // follow links forward, skipping tasks past the target, which the order puts out of reach
        const reached = new Set([from]);
        const stack = [from];
        for (let index = stack.pop(); index !== undefined; index = stack.pop()) {
            for (const next of nth(this.#nodes, index).outgoing) {
                if (next.index <= target && !reached.has(next.index)) {
                    reached.add(next.index);
                    stack.push(next.index);
                }
            }
        }

        return reached.has(target);
    }
}

/** Read the earliest start a task's links and bound allow, its own start at the least to push. */
function earliestOf<Task extends ScheduleTask>(
    node: ScheduleNode<Task>,
    task: Task,
    current: (index: number) => Task,
    mode: ScheduleMode,
    calendar: WorkCalendar,
): number {
    // pin a task its bound fixes, whatever its links ask
    const length = calendar.between(task.start, task.end);
    const { constraint } = task;
    if (constraint?.type === "must-start-on") {
        return calendar.align(constraint.at, "start");
    } else if (constraint?.type === "must-finish-on") {
        return startOf(constraint.at, length, calendar);
    }

    // start from its own start to push, or from its bound alone to run as soon as possible
    const isAnchored = mode === "push" || node.incoming.length === 0;
    const bound =
        constraint?.type === "start-no-earlier-than"
            ? calendar.align(constraint.at, "start")
            : constraint?.type === "finish-no-earlier-than"
              ? startOf(constraint.at, length, calendar)
              : Number.NEGATIVE_INFINITY;
    const own = isAnchored ? calendar.align(task.start, "start") : Number.NEGATIVE_INFINITY;

    // take the latest start any link into the task asks for
    return node.incoming.reduce(
        (earliest, { index, link }) =>
            Math.max(earliest, earliestStart(link, current(index), length, calendar)),
        Math.max(own, bound),
    );
}

/** Read the latest start a task's links and bound allow, the schedule's end at the most. */
function latestOf<Task extends ScheduleTask>(
    node: ScheduleNode<Task>,
    end: number,
    nodes: readonly ScheduleNode<Task>[],
    calendar: WorkCalendar,
): number {
    // bound the start by the task's own constraint, and by the schedule's end
    const length = calendar.between(node.task.start, node.task.end);
    const { constraint } = node.task;
    const bound =
        constraint?.type === "start-no-later-than" || constraint?.type === "must-start-on"
            ? calendar.align(constraint.at, "start")
            : constraint?.type === "finish-no-later-than" || constraint?.type === "must-finish-on"
              ? startOf(constraint.at, length, calendar)
              : Number.POSITIVE_INFINITY;

    // take the earliest start any link out of the task allows, each later task set before
    return node.outgoing.reduce(
        (latest, { index, link }) => {
            const to = nth(nodes, index);
            const toLength = calendar.between(to.task.start, to.task.end);

            return Math.min(latest, latestStart(link, to.latest, toLength, length, calendar));
        },
        Math.min(startOf(end, length, calendar), bound),
    );
}

/** Read the start a link asks of the task waiting on it, by the type and the lag. */
function earliestStart(
    link: ScheduleLink,
    from: ScheduleTask,
    length: number,
    calendar: WorkCalendar,
): number {
    // wait on an edge of the earlier task, the waiting edge being the start or the end
    const edges = ScheduleLink.edges(link.type);
    const edge = calendar.align(
        calendar.add(edges.from === "end" ? from.end : from.start, link.lag),
        edges.to,
    );

    return edges.to === "start" ? edge : startOf(edge, length, calendar);
}

/** Read the latest start a link allows the task it starts from, by the waiting task's start. */
function latestStart(
    link: ScheduleLink,
    toStart: number,
    toLength: number,
    length: number,
    calendar: WorkCalendar,
): number {
    // read the waiting edge, then the earlier task's edge the lag before it
    const edges = ScheduleLink.edges(link.type);
    const edge = calendar.add(
        edges.to === "start" ? toStart : calendar.add(toStart, toLength),
        -link.lag,
    );

    return edges.from === "end" ? startOf(edge, length, calendar) : calendar.align(edge, "start");
}

/** Read the start of a task of a length that ends at an instant, both on working time. */
function startOf(end: number, length: number, calendar: WorkCalendar): number {
    return calendar.align(calendar.add(calendar.align(end, "end"), -length), "start");
}

/** Turn a link around, for the pair check that ignores direction. */
function reversed(link: ScheduleLink): ScheduleLink {
    return { ...link, from: link.to, to: link.from };
}

/** Order tasks so each comes after the ones it waits on, by Kahn's algorithm in given order. */
function ordered<Task extends ScheduleTask>(
    tasks: readonly Task[],
    links: readonly ScheduleLink[],
): Task[] {
    // index the given tasks, refusing a repeated key
    const indexes = new Map<string, number>();
    for (const [index, task] of tasks.entries()) {
        if (indexes.has(task.key)) {
            throw new RangeError(`schedule task '${task.key}' is repeated`);
        }
        indexes.set(task.key, index);
    }

    // count each task's links in and list its links out, refusing a link to a missing task
    const waiting = tasks.map(() => 0);
    const outgoing = tasks.map((): number[] => []);
    for (const link of links) {
        const from = indexes.get(link.from);
        const to = indexes.get(link.to);
        if (from === undefined || to === undefined) {
            throw new RangeError(
                `schedule link '${link.from}' to '${link.to}' joins a missing task`,
            );
        }
        waiting[to] = nth(waiting, to) + 1;
        nth(outgoing, from).push(to);
    }

    // take tasks whose links are all met, freeing the tasks that wait on them
    const ready = [...waiting.keys()].filter((index) => nth(waiting, index) === 0);
    for (let cursor = 0; cursor < ready.length; cursor += 1) {
        for (const to of nth(outgoing, nth(ready, cursor))) {
            waiting[to] = nth(waiting, to) - 1;
            if (nth(waiting, to) === 0) {
                ready.push(to);
            }
        }
    }

    // refuse a cycle, which leaves its tasks waiting
    if (ready.length < tasks.length) {
        const stuck = tasks.filter((_, index) => nth(waiting, index) > 0).map((task) => task.key);
        throw new RangeError(`schedule links form a cycle through '${stuck.join("', '")}'`);
    }

    return ready.map((index) => nth(tasks, index));
}
