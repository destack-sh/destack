import { defineExample } from "@destack/package/declare";
import {
    columnFilteringFeature,
    columnVisibilityFeature,
    createColumnHelper,
    createFilteredRowModel,
    createPaginatedRowModel,
    createSortedRowModel,
    filterFn_includesString,
    globalFilteringFeature,
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
