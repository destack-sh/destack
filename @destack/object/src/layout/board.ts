import { type Localization, t } from "@destack/locale";
import { type JsonObject, schema } from "@destack/schema";
import type { Field } from "../field/field.ts";
import { ObjectError } from "../error/error.ts";
import { Choice } from "../presentation/choice.ts";

/** One column of a board: a value of its field and the objects with it. */
export interface BoardGroup {
    /** The value, null for the objects without one of the listed. */
    readonly choice: Choice;
    /** The objects with the value. */
    readonly rows: readonly JsonObject[];
}

/** One column per value of a field. */
const boardLayout = schema.object({
    /** The kind. */
    kind: schema.literal("board"),
    /** The field grouping the objects: an enum, a state or a reference. */
    by: schema.string().min(1),
    /** Whether the columns without objects show. */
    empty: schema.enum(["show", "hide"]),
});
/** One column per value of a field. */
export type BoardLayout = schema.Infer<typeof boardLayout>;

/** One column per value of a field. */
export const BoardLayout = Object.assign(boardLayout, {
    /** List the columns of a board by an enum or a state, absent for one by a reference. */
    choices(
        layout: BoardLayout,
        fields: Readonly<Record<string, Field>>,
        localization: Localization,
    ): Choice[] | undefined {
        // refuse a field that groups into no columns
        const field = fields[layout.by];
        if (field === undefined || !BoardLayout.isGrouping(field)) {
            throw ObjectError.of(
                "DECLARATION_BROKEN",
                `board field '${layout.by}' is invalid: it is not an enum, a state or a reference to ordered objects`,
            );
        }

        return field.choices?.map((each) => Choice.of(each, localization));
    },

    /** Report whether a field groups objects: an enum, a state or a reference to ordered ones. */
    isGrouping(field: Field): boolean {
        return field.choices !== undefined || field.target?.().ordering !== undefined;
    },

    /** Group objects into a board's columns in order, those in none of them under no value. */
    group(
        layout: BoardLayout,
        rows: readonly JsonObject[],
        choices: readonly Choice[],
        localization: Localization,
    ): BoardGroup[] {
        // group by value, then leave out the empty columns a layout hides
        const groups = choices.map((choice) => ({
            choice,
            rows: rows.filter((row) => row[layout.by] === choice.value),
        }));
        const shown =
            layout.empty === "show" ? groups : groups.filter((group) => group.rows.length > 0);

        // put the objects without a listed value first
        const rest = rows.filter((row) =>
            choices.every((choice) => choice.value !== row[layout.by]),
        );
        const none: Choice = {
            value: null,
            title: localization.render(t.context("option")`No value`),
        };

        return rest.length === 0 ? shown : [{ choice: none, rows: rest }, ...shown];
    },

    /** Extend a list query to read the objects in column order. */
    query<Query extends { readonly orderBy?: object }>(layout: BoardLayout, query: Query): Query {
        return { ...query, orderBy: { [layout.by]: "asc", ...query.orderBy } };
    },
});
