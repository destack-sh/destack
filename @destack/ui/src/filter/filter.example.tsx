import { Condition, type JsonCondition, now } from "@destack/db";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal, For, useEnvironment } from "@destack/view";
import { Item, ItemContent, ItemTitle } from "../item/index.ts";
import { type FilterFields, matchesFilter } from "./control.ts";
import {
    Filter,
    FilterClear,
    FilterField,
    FilterItem,
    FilterGroup,
    FilterList,
    FilterMatch,
    FilterMenu,
    FilterAdd,
    FilterRemove,
    FilterValue,
} from "./filter.tsx";

/** The styles of the examples. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[4],
        width: "100%",
    },
});

/** A day, in milliseconds. */
const DAY = 86_400_000;

/** The states of a task. */
const STATES = [
    { value: "open", title: "Open", color: "gray" },
    { value: "doing", title: "Doing", color: "blue" },
    { value: "done", title: "Done", color: "green" },
] as const;

/** The people of a task. */
const PEOPLE = ["Ada", "Grace", "Barbara"].map((name) => ({ value: name, title: name }));

/** The fields of a task a person filters by. */
const FIELDS: FilterFields = {
    title: { label: "Title", type: { kind: "text" } },
    state: { label: "State", type: { kind: "choice" }, choices: STATES },
    assignee: { label: "Assignee", type: { kind: "person" }, choices: PEOPLE },
    estimate: { label: "Estimate", type: { kind: "number", minimum: 0 } },
    due: { label: "Due", type: { kind: "time" } },
    isPinned: { label: "Pinned", type: { kind: "boolean" } },
};

/** Tasks of Ada's that are open, or short and due this week. */
const NESTED: JsonCondition = Condition.json({
    AND: [
        { assignee: "Ada" },
        {
            OR: [
                { state: "open" },
                {
                    AND: [
                        { estimate: { lte: 2 } },
                        {
                            due: {
                                gte: now.start("week"),
                                lt: now.start("week").plus({ days: 7 }),
                            },
                        },
                    ],
                },
            ],
        },
    ],
});

/** Some tasks, due around now. */
function tasksAround(today: number) {
    return [
        { title: "Book the train", state: "open", assignee: "Ada", estimate: 1, due: today },
        { title: "Pack", state: "doing", assignee: "Grace", estimate: 2, due: today + 2 * DAY },
        { title: "Print tickets", state: "done", assignee: "Ada", estimate: 1, due: today - DAY },
        {
            title: "Book the hotel",
            state: "open",
            assignee: null,
            estimate: 3,
            due: today + 9 * DAY,
        },
    ];
}

/** Tasks narrowed by what a person filters by, as a query would be. */
export const filterTasks = defineExample({
    of: Filter,
    name: "tasks",
    description: "tasks narrowed by what a person filters by, as a query would be",
    layout: "padded",
    render: () => {
        // match the tasks as the engine does
        const { clock, timeZone } = useEnvironment();
        const moment = { now: clock.now(), timeZone };
        const [where, setWhere] = createSignal<JsonCondition>({ state: { in: ["open", "doing"] } });
        const tasks = tasksAround(moment.now);
        const shown = () => tasks.filter((task) => matchesFilter(where(), task, moment));

        return (
            <div {...style.attrs(styles.page)}>
                <Filter fields={FIELDS} value={where()} onValueChange={setWhere} />
                <div>
                    <For each={shown()}>
                        {(task) => (
                            <Item>
                                <ItemContent>
                                    <ItemTitle>{task.title}</ItemTitle>
                                </ItemContent>
                            </Item>
                        )}
                    </For>
                </div>
                <output>{JSON.stringify(where())}</output>
            </div>
        );
    },
});

/** Conditions of every kind, with a set of which any holds. */
export const filterSets = defineExample({
    of: Filter,
    name: "sets",
    description: "conditions of every kind, with a set of which any holds",
    layout: "padded",
    render: () => (
        <Filter
            fields={FIELDS}
            defaultValue={{
                AND: [
                    { title: { ilike: "%book%" } },
                    { estimate: { gte: 1, lte: 3 } },
                    { due: { lt: Date.UTC(2026, 10, 30) } },
                    { assignee: { isNull: true } },
                    { OR: [{ state: "open" }, { isPinned: true }] },
                ],
            }}
        >
            <FilterList />
            <FilterAdd groups />
            <FilterMatch />
            <FilterClear />
        </Filter>
    ),
});

/** Conditions below each other with how they combine, and all of them under one button. */
export const filterBuilder = defineExample({
    of: FilterGroup,
    name: "builder",
    description: "conditions below each other with how they combine, and all under one button",
    layout: "padded",
    render: () => (
        <div {...style.attrs(styles.page)}>
            <Filter fields={FIELDS} defaultValue={NESTED}>
                <FilterGroup />
            </Filter>
            <Filter fields={FIELDS} defaultValue={NESTED}>
                <FilterMenu />
            </Filter>
        </div>
    ),
});

/** Conditions laid out by an app, without their comparison. */
export const filterParts = defineExample({
    of: FilterItem,
    name: "parts",
    description: "conditions laid out by an app, without their comparison",
    layout: "padded",
    render: () => (
        <Filter fields={FIELDS} defaultValue={{ state: "open", assignee: "Ada" }}>
            <FilterAdd variant="outline">Narrow</FilterAdd>
            <FilterList>
                {() => (
                    <FilterItem>
                        <FilterValue words={(values) => <b>{values.map(String).join(" or ")}</b>} />
                        <FilterField />
                        <FilterRemove />
                    </FilterItem>
                )}
            </FilterList>
        </Filter>
    ),
});
