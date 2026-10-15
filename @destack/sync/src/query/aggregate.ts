import { defineSchema, schema } from "@destack/schema";
import type { ConditionOf, Model, Scalar } from "@destack/db";

/** One measure of a group: its row count, or a function of one column. */
export const Measure = defineSchema(
    schema.discriminatedUnion("function", [
        schema.object({
            /** Count the rows. */
            function: schema.literal("count"),
        }),
        schema.object({
            /** Add up, average, or take the least or greatest value. */
            function: schema.enum(["sum", "avg", "min", "max"]),
            /** The measured column. */
            column: schema.string().min(1),
        }),
    ]),
);
/** One measure of a group: its row count, or a function of one column. */
export type Measure = schema.Infer<typeof Measure>;

/** Aggregates of a query's rows per group. */
export const Aggregate = defineSchema(
    schema.object({
        /** The group columns, one group when absent. */
        groupBy: schema.array(schema.string().min(1)).exactOptional(),
        /** The measures of each group, by name. */
        values: schema.record(schema.string(), Measure),
    }),
);
/** Aggregates of a query's rows per group. */
export type Aggregate = schema.Infer<typeof Aggregate>;

/** Measures of a model's rows per group. */
export type AggregateOptions<Entity extends Model> = {
    /** The condition the measured rows meet. */
    readonly where?: ConditionOf<Entity>;
} & Aggregate;

/** One group of an aggregate's rows: its group values and its measures, by name. */
export type AggregateRow<Options extends Aggregate = Aggregate> = {
    /** The values of the group columns. */
    readonly group: { readonly [Name in GroupColumnOf<Options>]: Scalar };
    /** The measures, by name. */
    readonly values: {
        readonly [Name in keyof Options["values"]]: Options["values"][Name] extends {
            readonly function: "count";
        }
            ? number
            : Scalar;
    };
};

/** The group columns of an aggregate, any column for an aggregate not known statically. */
type GroupColumnOf<Options extends Aggregate> = Aggregate extends Options
    ? string
    : Options extends {
            readonly groupBy: readonly (infer Column extends string)[];
        }
      ? Column
      : never;
