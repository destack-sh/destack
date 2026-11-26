import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { defineExample } from "@destack/package/declare";
import {
    aggregationFn_sum,
    cellSelectionFeature,
    columnOrderingFeature,
    columnFilteringFeature,
    columnPinningFeature,
    columnResizingFeature,
    columnSizingFeature,
    columnGroupingFeature,
    columnVisibilityFeature,
    createColumnHelper,
    createExpandedRowModel,
    createFilteredRowModel,
    createGroupedRowModel,
    createPaginatedRowModel,
    createSortedRowModel,
    filterFn_includesString,
    globalFilteringFeature,
    type ExpandedState,
    rowAggregationFeature,
    rowExpandingFeature,
    rowPaginationFeature,
    rowSelectionFeature,
    rowSortingFeature,
    sortFn_alphanumeric,
    sortFn_basic,
    tableFeatures,
} from "./index.ts";
import { createMemo, createSignal, handleOnPage, type JSX, useEnvironment } from "@destack/view";
import type { JsonCondition } from "@destack/db";
import { present } from "@destack/schema";
import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { ActionButton, ActionMenu, items, removeItems } from "../action/index.ts";
import type {} from "./actions.ts";
import { Filter, type FilterFields, matchesFilter } from "../filter/index.ts";
import { SearchField } from "../search-field/index.ts";
import { SortableMove } from "../sortable/index.ts";
import { Cluster } from "../cluster/index.ts";
import { Stack } from "../stack/index.ts";
import {
    DataTable,
    DataTableColumnHeader,
    DataTablePagination,
    DataTableViewOptions,
    DataTableBulkBar,
    expansionColumn,
    orderColumn,
    selectionColumn,
} from "./data-table.tsx";
import { type DataTableCellChange, DataTableEditor } from "./control.ts";
import { createTable } from "./table.ts";

/** The styles of the examples' tables. */
const styles = style.create({
    margin: {
        // leave room for the grip and the checkbox before each row
        paddingInlineStart: space[5],
    },
    search: {
        width: "16rem",
    },
    scrolling: {
        maxHeight: "24rem",
    },
});

/** A note the table lists. */
interface NoteRow {
    /** The note's id. */
    readonly id: string;
    /** The note's title. */
    readonly title: string;
    /** The number of words. */
    readonly words: number;
}

/** The features of the notes table: sorting, filtering, paging, selecting and hiding columns. */
const FEATURES = tableFeatures({
    rowSortingFeature,
    sortedRowModel: createSortedRowModel(),
    sortFns: { alphanumeric: sortFn_alphanumeric, basic: sortFn_basic },
    columnFilteringFeature,
    globalFilteringFeature,
    filteredRowModel: createFilteredRowModel(),
    filterFns: { includesString: filterFn_includesString },
    rowPaginationFeature,
    paginatedRowModel: createPaginatedRowModel(),
    rowSelectionFeature,
    columnVisibilityFeature,
});

/** The column helper of the notes table. */
const COLUMN = createColumnHelper<typeof FEATURES, NoteRow>();

/** The columns of the notes table. */
const COLUMNS = COLUMN.columns([
    selectionColumn<typeof FEATURES, NoteRow>(),
    COLUMN.accessor("title", {
        header: (context) => <DataTableColumnHeader column={context.column} />,
        meta: { label: "Title" },
    }),
    COLUMN.accessor("words", {
        header: (context) => <DataTableColumnHeader column={context.column} />,
        meta: { label: "Words", align: "end" },
        enableGlobalFilter: false,
    }),
]);

/** A notebook's notes. */
const NOTES: NoteRow[] = [
    { id: "groceries", title: "Groceries", words: 18 },
    { id: "lisbon", title: "Trip to Lisbon", words: 940 },
    { id: "standup", title: "Standup notes", words: 312 },
    { id: "bread", title: "Rye bread recipe", words: 455 },
];

/** Render the notes table over some notes with a page size. */
function NotesTable(properties: {
    /** The notes. */
    readonly notes: NoteRow[];
    /** The rows on each page. */
    readonly pageSize: number;
    /** The page sizes the pagination offers, the pagination's by default. */
    readonly pageSizes?: readonly number[];
    /** Whether only the rows in view render. */
    readonly isVirtual?: boolean;
    /** The ids of the notes that cannot be selected. */
    readonly locked?: readonly string[];
    /** The ids of the notes selected at first. */
    readonly selected?: readonly string[];
}): JSX.Element {
    const table = createTable({
        features: FEATURES,
        columns: COLUMNS,
        get data() {
            return properties.notes;
        },
        getRowId: (row) => row.id,
        enableRowSelection: (row) => properties.locked?.includes(row.id) !== true,
        initialState: {
            pagination: { pageIndex: 0, pageSize: properties.pageSize },
            rowSelection: Object.fromEntries((properties.selected ?? []).map((id) => [id, true])),
        },
    });

    return (
        <Stack>
            <Cluster justify="between">
                <SearchField
                    aria-label="Search notes"
                    placeholder="Search notes"
                    xstyle={styles.search}
                    onValueChange={(text) => table.setGlobalFilter(text)}
                />
                <DataTableViewOptions table={table} />
            </Cluster>
            <DataTable
                table={table}
                isVirtual={properties.isVirtual}
                xstyle={properties.isVirtual === true && styles.scrolling}
            />
            <DataTablePagination table={table} pageSizes={properties.pageSizes} />
        </Stack>
    );
}

/** Notes in a table that sorts, filters, selects, hides columns and pages. */
export const dataTableDefault = defineExample({
    of: DataTable,
    name: "default",
    description: t`Notes in a table that sorts, filters, selects, hides columns and pages.`,
    layout: "padded",
    render: () => <NotesTable notes={NOTES} pageSize={10} />,
});

/** Notes in a table whose shared notes cannot be selected. */
export const dataTableDisabled = defineExample({
    of: DataTable,
    name: "disabled",
    description: t`Notes in a table whose shared notes cannot be selected, the others selected.`,
    layout: "padded",
    render: () => {
        // lock the shared notes, and select every other one
        const locked = NOTES.slice(1, 3).map((note) => note.id);
        const selected = NOTES.map((note) => note.id).filter((id) => !locked.includes(id));

        return <NotesTable notes={NOTES} pageSize={10} locked={locked} selected={selected} />;
    },
});

/** A notes table with no notes. */
export const dataTableEmpty = defineExample({
    of: DataTable,
    name: "empty",
    description: t`A notes table with no notes.`,
    layout: "padded",
    render: () => <NotesTable notes={[]} pageSize={10} />,
});

/** A thousand notes in a scrolling table that renders only the rows in view. */
export const dataTableVirtual = defineExample({
    of: DataTable,
    name: "virtual",
    description: t`A thousand notes in a scrolling table that renders only the rows in view.`,
    layout: "padded",
    render: () => (
        <NotesTable
            notes={Array.from({ length: 1000 }, (_, index) => ({
                id: `note-${index}`,
                title: `Note ${index + 1}`,
                words: (index * 37) % 1000,
            }))}
            pageSize={1000}
            pageSizes={[100, 1000]}
            isVirtual
        />
    ),
});

/** A note in a notebook the grouped table lists. */
interface FiledNote {
    /** The note's id. */
    readonly id: string;
    /** The notebook it is in. */
    readonly notebook: string;
    /** The note's title. */
    readonly title: string;
    /** The number of words. */
    readonly words: number;
}

/** The features of the grouped notes table: grouping by a column, summing and expanding. */
const GROUPED_FEATURES = tableFeatures({
    columnGroupingFeature,
    groupedRowModel: createGroupedRowModel(),
    rowAggregationFeature,
    aggregationFns: { sum: aggregationFn_sum },
    rowExpandingFeature,
    expandedRowModel: createExpandedRowModel(),
});

/** The column helper of the grouped notes table. */
const GROUPED_COLUMN = createColumnHelper<typeof GROUPED_FEATURES, FiledNote>();

/** The columns of the grouped notes table: the notebook groups, the words sum. */
const GROUPED_COLUMNS = GROUPED_COLUMN.columns([
    GROUPED_COLUMN.accessor("notebook", { header: "Notebook" }),
    GROUPED_COLUMN.accessor("title", { header: "Title" }),
    GROUPED_COLUMN.accessor("words", {
        header: "Words",
        aggregationFn: "sum",
        meta: { align: "end" },
    }),
]);

/** Notes filed in notebooks. */
const FILED: FiledNote[] = [
    { id: "groceries", notebook: "Home", title: "Groceries", words: 18 },
    { id: "bread", notebook: "Home", title: "Rye bread recipe", words: 455 },
    { id: "standup", notebook: "Work", title: "Standup notes", words: 312 },
    { id: "plan", notebook: "Work", title: "Quarter plan", words: 1204 },
    { id: "lisbon", notebook: "Trips", title: "Trip to Lisbon", words: 940 },
];

/** Render the notes grouped by notebook, some groups expanded. */
function GroupedNotes(properties: {
    /** The groups shown expanded, by row id, or every group. */
    readonly expanded: ExpandedState;
}): JSX.Element {
    const table = createTable({
        features: GROUPED_FEATURES,
        columns: GROUPED_COLUMNS,
        data: FILED,
        initialState: { grouping: ["notebook"], expanded: properties.expanded },
    });

    return <DataTable table={table} />;
}

/** Notes grouped by notebook, each group collapsed to its words sum. */
export const dataTableGrouped = defineExample({
    of: DataTable,
    name: "grouped",
    description: t`Notes grouped by notebook, each group collapsed to its words sum.`,
    layout: "padded",
    render: () => <GroupedNotes expanded={{}} />,
});

/** Notes grouped by notebook with every group expanded to its notes. */
export const dataTableGroupedExpanded = defineExample({
    of: DataTable,
    name: "groupedExpanded",
    description: t`Notes grouped by notebook with every group expanded to its notes.`,
    layout: "padded",
    render: () => <GroupedNotes expanded />,
});

/** A task with the subtasks it breaks into. */
interface TaskRow {
    /** The task's id. */
    readonly id: string;
    /** The task's title. */
    readonly title: string;
    /** The subtasks. */
    readonly subtasks?: readonly TaskRow[];
}

/** The features of the tasks table: expanding rows into their subtasks. */
const TASK_FEATURES = tableFeatures({
    rowExpandingFeature,
    expandedRowModel: createExpandedRowModel(),
});

/** The column helper of the tasks table. */
const TASK_COLUMN = createColumnHelper<typeof TASK_FEATURES, TaskRow>();

/** Tasks with subtasks in a table that expands each task to its subtasks. */
export const dataTableNested = defineExample({
    of: DataTable,
    name: "nested",
    description: t`Tasks with subtasks in a table that expands each task to its subtasks.`,
    layout: "padded",
    render: () => {
        const table = createTable({
            features: TASK_FEATURES,
            columns: TASK_COLUMN.columns([
                expansionColumn<typeof TASK_FEATURES, TaskRow>(),
                TASK_COLUMN.accessor("title", { header: "Task" }),
            ]),
            data: [
                {
                    id: "launch",
                    title: "Launch the site",
                    subtasks: [
                        { id: "copy", title: "Write the copy" },
                        { id: "deploy", title: "Deploy" },
                    ],
                },
                { id: "invoice", title: "Send invoices" },
            ],
            getRowId: (row) => row.id,
            getSubRows: (row) => (row.subtasks === undefined ? undefined : [...row.subtasks]),
            initialState: { expanded: { launch: true } },
        });

        return <DataTable table={table} />;
    },
});

/** A task the tasks table lists. */
interface LaunchTask {
    /** The task's key. */
    readonly id: string;
    /** The task's title. */
    readonly title: string;
    /** The task's status. */
    readonly status: (typeof LAUNCH_STATUSES)[number]["value"];
    /** The task's priority. */
    readonly priority: (typeof LAUNCH_PRIORITIES)[number]["value"];
    /** The days the task is estimated to take. */
    readonly estimate: number;
    /** The day the task is due, written YYYY-MM-DD. */
    readonly due: string;
    /** Whether the task waits on something outside the team. */
    readonly isBlocked: boolean;
}

/** The features of the tasks table: sized and pinned columns, selected rows and cells. */
const LAUNCH_FEATURES = tableFeatures({
    columnSizingFeature,
    columnResizingFeature,
    columnPinningFeature,
    columnOrderingFeature,
    rowSelectionFeature,
    cellSelectionFeature,
});

/** The statuses a task moves through. */
const LAUNCH_STATUSES = [
    { value: "backlog", label: "Backlog" },
    { value: "doing", label: "Doing" },
    { value: "done", label: "Done" },
] as const;

/** The priorities a task takes. */
const LAUNCH_PRIORITIES = [
    { value: "high", label: "High" },
    { value: "low", label: "Low" },
] as const;

/** The launch's tasks. */
const LAUNCH_TASKS: LaunchTask[] = [
    {
        id: "LAU-21",
        title: "Write the changelog",
        status: "backlog",
        priority: "low",
        estimate: 1,
        due: "2026-11-27",
        isBlocked: false,
    },
    {
        id: "LAU-14",
        title: "Record the demo",
        status: "doing",
        priority: "high",
        estimate: 2,
        due: "2026-11-24",
        isBlocked: false,
    },
    {
        id: "LAU-15",
        title: "Pricing page",
        status: "doing",
        priority: "high",
        estimate: 3,
        due: "2026-11-25",
        isBlocked: true,
    },
    {
        id: "LAU-11",
        title: "Sign-up flow copy",
        status: "done",
        priority: "low",
        estimate: 0.5,
        due: "2026-11-20",
        isBlocked: false,
    },
];

/** The column helper of the tasks table. */
const LAUNCH = createColumnHelper<typeof LAUNCH_FEATURES, LaunchTask>();

/** The label of each status. */
const STATUS_LABELS: Readonly<Record<LaunchTask["status"], string>> = {
    backlog: "Backlog",
    doing: "Doing",
    done: "Done",
};

/** The label of each priority. */
const PRIORITY_LABELS: Readonly<Record<LaunchTask["priority"], string>> = {
    high: "High",
    low: "Low",
};

/** Archive the tasks. */
const archive = defineAction({
    name: "archive-tasks",
    title: t.context("action")`Archive`,
    input: { items },
});

/** Render the launch's tasks with a filter, editable titles and row actions. */
function LaunchTasksTable(): JSX.Element {
    // store the tasks, changing the cells people edit or paste
    const [tasks, setTasks] = createSignal(LAUNCH_TASKS);
    const [where, setWhere] = createSignal<JsonCondition>({});
    const { clock, timeZone } = useEnvironment();
    const moment = { now: clock.now(), timeZone };
    const shown = createMemo(() =>
        tasks().filter((task) => matchesFilter(where(), { ...task }, moment)),
    );
    const fields: FilterFields = {
        status: {
            label: "Status",
            type: { kind: "choice" },
            choices: LAUNCH_STATUSES.map(({ value, label }) => ({ value, title: label })),
            count: (value) => tasks().filter(({ status }) => status === value).length,
        },
        priority: {
            label: "Priority",
            type: { kind: "choice" },
            choices: LAUNCH_PRIORITIES.map(({ value, label }) => ({ value, title: label })),
            count: (value) => tasks().filter(({ priority }) => priority === value).length,
        },
        estimate: { label: "Estimate", type: { kind: "number", minimum: 0 } },
        isBlocked: { label: "Blocked", type: { kind: "boolean" } },
    };
    const change = (changes: readonly DataTableCellChange[]) =>
        setTasks((current) =>
            current.map((task) => {
                // copy only the tasks that change, so the others reuse their rows' elements
                const changed = changes.filter((each) => each.row === task.id);

                return changed.length === 0
                    ? task
                    : {
                          ...task,
                          ...Object.fromEntries(changed.map((each) => [each.column, each.value])),
                      };
            }),
        );
    const remove = (ids: readonly string[]): void => {
        setTasks((current) => current.filter((task) => !ids.includes(task.id)));
    };
    handleOnPage(archive, { run: ({ values }) => remove(values.items) });
    handleOnPage(removeItems, { run: ({ values }) => remove(values.items) });
    const columns = LAUNCH.columns([
        LAUNCH.display(orderColumn()),
        LAUNCH.display(selectionColumn()),
        LAUNCH.accessor("id", { header: "Task", size: 96, enableResizing: false }),
        LAUNCH.accessor("title", {
            header: "Title",
            size: 240,
            meta: { label: "Title", editor: DataTableEditor.text() },
        }),
        LAUNCH.accessor("status", {
            header: "Status",
            size: 120,
            meta: { label: "Status", editor: DataTableEditor.select(LAUNCH_STATUSES) },
            cell: (context) => STATUS_LABELS[context.getValue()],
        }),
        LAUNCH.accessor("priority", {
            header: "Priority",
            size: 120,
            meta: { label: "Priority", editor: DataTableEditor.select(LAUNCH_PRIORITIES) },
            cell: (context) => PRIORITY_LABELS[context.getValue()],
        }),
        LAUNCH.accessor("estimate", {
            header: "Estimate",
            size: 96,
            meta: { label: "Estimate", editor: DataTableEditor.number(), summary: "sum" },
        }),
        LAUNCH.accessor("due", {
            header: "Due",
            size: 128,
            meta: { label: "Due", editor: DataTableEditor.date(), summary: "min" },
        }),
        LAUNCH.accessor("isBlocked", {
            header: "Blocked",
            size: 88,
            meta: { label: "Blocked", editor: DataTableEditor.checkbox(), summary: "count" },
        }),
    ]);
    const table = createTable({
        features: LAUNCH_FEATURES,
        columns,
        get data() {
            return shown();
        },
        getRowId: (row) => row.id,
        enableRowSelection: true,
        columnResizeMode: "onChange",
        initialState: { columnPinning: { start: ["order", "select", "id"], end: [] } },
    });

    return (
        <Stack xstyle={styles.margin}>
            <Filter fields={fields} value={where()} onValueChange={setWhere} />
            <DataTableBulkBar table={table}>
                <ActionButton action={archive} variant="outline" size="sm" />
            </DataTableBulkBar>
            <ActionMenu />
            <DataTable
                table={table}
                onCellsChange={change}
                onRowMove={(move) =>
                    setTasks((current) =>
                        SortableMove.place(
                            current,
                            present(
                                current.find((task) => task.id === move.id),
                                `the task '${move.id}'`,
                            ),
                            move,
                            (task) => task.id,
                        ),
                    )
                }
            />
        </Stack>
    );
}

/** A launch's tasks with a filter, sized and pinned columns, editable titles and actions. */
export const dataTableTasks = defineExample({
    of: DataTable,
    name: "tasks",
    description: t`A launch's tasks with a filter, resizable pinned columns, editable titles and row actions.`,
    layout: "padded",
    render: () => <LaunchTasksTable />,
});
