import { schema } from "@destack/schema";

/** The most fields a table shows beside the title when the layout names none. */
const DEFAULT_FIELDS = 4;

/** One row per object. */
const tableLayout = schema.object({
    /** The kind. */
    kind: schema.literal("table"),
    /** The columns, the default columns when absent. */
    fields: schema.array(schema.string().min(1)).exactOptional(),
});
/** One row per object. */
export type TableLayout = schema.Infer<typeof tableLayout>;

/** One row per object. */
export const TableLayout = Object.assign(tableLayout, {
    /** Read the fields a table shows beside the title: the layout's, else the first shown ones. */
    columns(layout: TableLayout, shown: readonly string[]): readonly string[] {
        return layout.fields ?? shown.slice(0, DEFAULT_FIELDS);
    },
});
