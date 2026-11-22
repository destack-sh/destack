import * as style from "@destack/style";
import { defineExample } from "@destack/package/declare";
import {
    aggregationFn_sum,
    columnFacetingFeature,
    columnFilteringFeature,
    columnPinningFeature,
    columnResizingFeature,
    columnSizingFeature,
    createFacetedRowModel,
    createFacetedUniqueValues,
    filterFn_arrHas,
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
import { createSignal, type JSX } from "@destack/view";
import { DropdownMenuItem } from "../dropdown-menu/index.ts";
import { Cluster } from "../cluster/index.ts";
import { Stack } from "../stack/index.ts";
import {
    DataTable,
    DataTableColumnHeader,
    DataTableEditableCell,
    DataTableFacetedFilter,
    DataTableFilter,
    DataTableRowActions,
    DataTablePagination,
    DataTableViewOptions,
    expansionColumn,
    selectionColumn,
} from "./data-table.tsx";
import { createTable } from "./table.ts";

/** The styles of the examples' tables. */
const styles = style.create({
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
        header: (context) => <DataTableColumnHeader column={context.column} title="Title" />,
    }),
    COLUMN.accessor("words", {
        header: (context) => <DataTableColumnHeader column={context.column} title="Words" />,
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
    /** The height each row is estimated at, which renders only the rows in view. */
    readonly rowHeight?: number;
    /** The ids of the notes that cannot be selected. */
    readonly locked?: readonly string[];
}): JSX.Element {
    const table = createTable({
        features: FEATURES,
        columns: COLUMNS,
        get data() {
            return properties.notes;
        },
        getRowId: (row) => row.id,
        enableRowSelection: (row) => properties.locked?.includes(row.id) !== true,
        initialState: { pagination: { pageIndex: 0, pageSize: properties.pageSize } },
    });

    return (
        <Stack>
            <Cluster justify="between">
                <DataTableFilter table={table} placeholder="Filter notes" />
                <DataTableViewOptions table={table} />
            </Cluster>
            <DataTable
                table={table}
                rowHeight={properties.rowHeight}
                xstyle={properties.rowHeight !== undefined && styles.scrolling}
            />
            <DataTablePagination table={table} />
        </Stack>
    );
}

/** Notes in a table that sorts, filters, selects, hides columns and pages. */
export const dataTableNotes = defineExample({
    of: DataTable,
    name: "notes",
    description: "notes in a table that sorts, filters, selects, hides columns and pages",
    layout: "padded",
    render: () => <NotesTable notes={NOTES} pageSize={10} />,
});

/** Notes in a table whose shared notes cannot be selected. */
export const dataTableNotesLocked = defineExample({
    of: DataTable,
    name: "notes-locked",
    description: "notes in a table whose shared notes cannot be selected",
    layout: "padded",
    render: () => (
        <NotesTable notes={NOTES} pageSize={10} locked={NOTES.slice(1, 3).map((note) => note.id)} />
    ),
});

/** A notes table with no notes. */
export const dataTableNotesEmpty = defineExample({
    of: DataTable,
    name: "notes-empty",
    description: "a notes table with no notes",
    layout: "padded",
    render: () => <NotesTable notes={[]} pageSize={10} />,
});

/** A thousand notes in a scrolling table that renders only the rows in view. */
export const dataTableNotesVirtual = defineExample({
    of: DataTable,
    name: "notes-virtual",
    description: "a thousand notes in a scrolling table that renders only the rows in view",
    layout: "padded",
    render: () => (
        <NotesTable
            notes={Array.from({ length: 1000 }, (_, index) => ({
                id: `note-${index}`,
                title: `Note ${index + 1}`,
                words: (index * 37) % 1000,
            }))}
            pageSize={1000}
            rowHeight={40}
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
    GROUPED_COLUMN.accessor("words", { header: "Words", aggregationFn: "sum" }),
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
export const dataTableNotesGrouped = defineExample({
    of: DataTable,
    name: "notes-grouped",
    description: "notes grouped by notebook, each group collapsed to its words sum",
    layout: "padded",
    render: () => <GroupedNotes expanded={{}} />,
});

/** Notes grouped by notebook with every group expanded to its notes. */
export const dataTableNotesGroupedExpanded = defineExample({
    of: DataTable,
    name: "notes-grouped-expanded",
    description: "notes grouped by notebook with every group expanded to its notes",
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
export const dataTableTasksNested = defineExample({
    of: DataTable,
    name: "tasks-nested",
    description: "tasks with subtasks in a table that expands each task to its subtasks",
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
}

/** The features of the tasks table: faceted filters, sized and pinned columns. */
const LAUNCH_FEATURES = tableFeatures({
    columnFilteringFeature,
    filteredRowModel: createFilteredRowModel(),
    filterFns: { arrHas: filterFn_arrHas },
    columnFacetingFeature,
    facetedRowModel: createFacetedRowModel(),
    facetedUniqueValues: createFacetedUniqueValues(),
    columnSizingFeature,
    columnResizingFeature,
    columnPinningFeature,
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
    { id: "LAU-21", title: "Write the changelog", status: "backlog", priority: "low" },
    { id: "LAU-14", title: "Record the demo", status: "doing", priority: "high" },
    { id: "LAU-15", title: "Pricing page", status: "doing", priority: "high" },
    { id: "LAU-11", title: "Sign-up flow copy", status: "done", priority: "low" },
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

/** Render the launch's tasks with faceted filters, editable titles and row actions. */
function LaunchTasksTable(): JSX.Element {
    // keep the tasks, renaming one in place
    const [tasks, setTasks] = createSignal(LAUNCH_TASKS);
    const rename = (id: string, title: string) =>
        setTasks((current) => current.map((task) => (task.id === id ? { ...task, title } : task)));
    const columns = LAUNCH.columns([
        LAUNCH.accessor("id", { header: "Task", size: 96, enableResizing: false }),
        LAUNCH.accessor("title", {
            header: "Title",
            size: 240,
            cell: (context) => (
                <DataTableEditableCell
                    value={context.getValue()}
                    label="Title"
                    onCommit={(title) => rename(context.row.original.id, title)}
                />
            ),
        }),
        LAUNCH.accessor("status", {
            header: "Status",
            size: 120,
            filterFn: "arrHas",
            cell: (context) => STATUS_LABELS[context.getValue()],
        }),
        LAUNCH.accessor("priority", {
            header: "Priority",
            size: 120,
            filterFn: "arrHas",
            cell: (context) => PRIORITY_LABELS[context.getValue()],
        }),
        LAUNCH.display({
            id: "actions",
            size: 48,
            enableResizing: false,
            cell: () => (
                <DataTableRowActions>
                    <DropdownMenuItem>Copy link</DropdownMenuItem>
                    <DropdownMenuItem>Archive</DropdownMenuItem>
                </DataTableRowActions>
            ),
        }),
    ]);
    const table = createTable({
        features: LAUNCH_FEATURES,
        columns,
        get data() {
            return tasks();
        },
        getRowId: (row) => row.id,
        columnResizeMode: "onChange",
        initialState: { columnPinning: { start: ["id"], end: [] } },
    });

    return (
        <Stack>
            <Cluster>
                <DataTableFacetedFilter
                    table={table}
                    column="status"
                    title="Status"
                    options={LAUNCH_STATUSES}
                />
                <DataTableFacetedFilter
                    table={table}
                    column="priority"
                    title="Priority"
                    options={LAUNCH_PRIORITIES}
                />
            </Cluster>
            <DataTable table={table} />
        </Stack>
    );
}

/** A launch's tasks with faceted filters, sized and pinned columns, editable titles and actions. */
export const dataTableTasks = defineExample({
    of: DataTable,
    name: "tasks",
    description:
        "a launch's tasks with faceted filters, resizable pinned columns, editable titles and row actions",
    layout: "padded",
    render: () => <LaunchTasksTable />,
});
