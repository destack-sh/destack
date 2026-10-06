import { defineExample } from "@destack/package/declare";
import { DataTable, type DataTableColumn } from "./data-table.tsx";

/** A note the table lists. */
interface NoteRow {
    /** The note's id. */
    readonly id: string;
    /** The note's title. */
    readonly title: string;
    /** The number of words. */
    readonly words: number;
}

/** The columns of the notes table. */
const COLUMNS: readonly DataTableColumn<NoteRow>[] = [
    {
        id: "title",
        header: "Title",
        cell: (row) => row.title,
        sortValue: (row) => row.title,
        filterValue: (row) => row.title,
    },
    {
        id: "words",
        header: "Words",
        cell: (row) => String(row.words),
        sortValue: (row) => row.words,
    },
];

/** A notebook's notes. */
const NOTES: readonly NoteRow[] = [
    { id: "groceries", title: "Groceries", words: 18 },
    { id: "lisbon", title: "Trip to Lisbon", words: 940 },
    { id: "standup", title: "Standup notes", words: 312 },
    { id: "bread", title: "Rye bread recipe", words: 455 },
];

/** Notes in a table that sorts, filters, selects and pages. */
export const dataTableNotes = defineExample({
    of: DataTable,
    name: "notes",
    description: "notes in a table that sorts, filters, selects and pages",
    render: () => (
        <DataTable
            rows={NOTES}
            columns={COLUMNS}
            rowId={(row) => row.id}
            isSelectable
            filterPlaceholder="Filter notes"
        />
    ),
});
