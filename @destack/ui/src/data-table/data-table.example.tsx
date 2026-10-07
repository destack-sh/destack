import { defineExample } from "@destack/package/declare";
import {
    aggregationFn_sum,
    columnFilteringFeature,
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
} from "@tanstack/table-core";
import type { JSX } from "@destack/view";
import {
    DataTable,
    DataTableColumnHeader,
    DataTableFilter,
    DataTablePagination,
    DataTableViewOptions,
    expansionColumn,
    selectionColumn,
} from "./data-table.tsx";
import { createTable } from "./table.ts";

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
}): JSX.Element {
    const table = createTable({
        features: FEATURES,
        columns: COLUMNS,
        get data() {
            return properties.notes;
        },
        getRowId: (row) => row.id,
        initialState: { pagination: { pageIndex: 0, pageSize: properties.pageSize } },
    });

    return (
        <div>
            <DataTableFilter table={table} placeholder="Filter notes" />
            <DataTableViewOptions table={table} />
            <DataTable table={table} rowHeight={properties.rowHeight} />
            <DataTablePagination table={table} />
        </div>
    );
}

/** Notes in a table that sorts, filters, selects, hides columns and pages. */
export const dataTableNotes = defineExample({
    of: DataTable,
    name: "notes",
    description: "notes in a table that sorts, filters, selects, hides columns and pages",
    render: () => <NotesTable notes={NOTES} pageSize={10} />,
});

/** A notes table with no notes. */
export const dataTableNotesEmpty = defineExample({
    of: DataTable,
    name: "notes-empty",
    description: "a notes table with no notes",
    render: () => <NotesTable notes={[]} pageSize={10} />,
});

/** A thousand notes in a scrolling table that renders only the rows in view. */
export const dataTableNotesVirtual = defineExample({
    of: DataTable,
    name: "notes-virtual",
    description: "a thousand notes in a scrolling table that renders only the rows in view",
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
    /** The notebook holding it. */
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
    render: () => <GroupedNotes expanded={{}} />,
});

/** Notes grouped by notebook with every group expanded to its notes. */
export const dataTableNotesGroupedExpanded = defineExample({
    of: DataTable,
    name: "notes-grouped-expanded",
    description: "notes grouped by notebook with every group expanded to its notes",
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
