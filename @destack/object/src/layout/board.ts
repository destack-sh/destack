import { type JsonObject, schema } from "@destack/schema";
import type { Field } from "../field/field.ts";
import { ObjectError } from "../error/error.ts";

/** One column of a board: a value of its field and the objects with it. */
export interface BoardGroup {
    /** The value. */
    readonly value: string;
    /** The objects with the value. */
    readonly rows: readonly JsonObject[];
}

/** One column per value of a field. */
const boardLayout = schema.object({
    /** The kind. */
    kind: schema.literal("board"),
    /** The enum or state field grouping the objects. */
    by: schema.string().min(1),
    /** Whether the columns without objects show. */
    empty: schema.enum(["show", "hide"]),
});
/** One column per value of a field. */
export type BoardLayout = schema.Infer<typeof boardLayout>;

/** One column per value of a field. */
export const BoardLayout = Object.assign(boardLayout, {
    /** Read the values a board groups by: an enum's in order, or a state machine's as reached. */
    columns(layout: BoardLayout, fields: Readonly<Record<string, Field>>): string[] {
        // take an enum's values
        const field = fields[layout.by];
        const machine = field?.machine;
        if (field?.source.builder === "enum") {
            return [...field.source.arguments[0]];
        }
        // take the states from the initial one in transition order
        else if (machine !== undefined) {
            const reached = Object.values(machine.transitions).flatMap((each) => [
                ...each.from,
                each.to,
            ]);

            return [...new Set([machine.initial, ...reached])];
        }
        // refuse a field of another type
        else {
            throw ObjectError.of(
                "DECLARATION_BROKEN",
                `board field '${layout.by}' is invalid: it is not an enum or a state`,
            );
        }
    },

    /** Group objects into the board's columns, in the columns' order. */
    group(
        layout: BoardLayout,
        rows: readonly JsonObject[],
        fields: Readonly<Record<string, Field>>,
    ): BoardGroup[] {
        // group by value, then leave out the empty columns a layout hides
        const groups = BoardLayout.columns(layout, fields).map((value) => ({
            value,
            rows: rows.filter((row) => row[layout.by] === value),
        }));

        return layout.empty === "show" ? groups : groups.filter((group) => group.rows.length > 0);
    },

    /** Extend a list query to read the objects in column order. */
    query<Query extends { readonly orderBy?: object }>(layout: BoardLayout, query: Query): Query {
        return { ...query, orderBy: { [layout.by]: "asc", ...query.orderBy } };
    },
});
