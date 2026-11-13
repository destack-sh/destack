import { t, type Message } from "@destack/locale";
import type { TASK_STATES } from "../object/index.ts";

/** Where a task stands. */
export type TaskState = (typeof TASK_STATES)[number];

/** The states a board shows as columns, in workflow order. */
export const BOARD_STATES = [
    "backlog",
    "todo",
    "started",
    "done",
] as const satisfies readonly TaskState[];

/** The title of each state. */
export const STATE_TITLES: Readonly<Record<TaskState, Message>> = {
    backlog: t`Backlog`,
    todo: t`Todo`,
    started: t`In progress`,
    done: t`Done`,
    canceled: t`Canceled`,
};

/** The state after each one in the workflow, none after done or canceled. */
export const NEXT_STATE: Readonly<Record<TaskState, TaskState | undefined>> = {
    backlog: "todo",
    todo: "started",
    started: "done",
    done: undefined,
    canceled: undefined,
};
