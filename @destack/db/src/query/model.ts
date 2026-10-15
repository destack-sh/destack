import { schema } from "@destack/schema";
import { type Scalar, Condition } from "./condition.ts";
import type { Extras } from "./namespace.ts";
import { OrderBy } from "./order.ts";
import { Expression } from "../expression/expression.ts";

/** What relational reads see of a table or object type: compared rows, results and relations. */
export interface Model {
    /** The fields conditions and orders compare. */
    readonly row: { readonly [Field in string]?: unknown };
    /** The fields each result has. */
    readonly value: { readonly [Field in string]?: unknown };
    /** The relations, by name. */
    readonly relations: { readonly [Name in string]?: One | Many };
}

/** A relation to at most one row. */
export interface One<Target extends Model = Model> {
    /** How many rows a row relates to. */
    readonly cardinality: "one";
    /** The related rows' model. */
    readonly target: Target;
}

/** A relation to any number of rows. */
export interface Many<Target extends Model = Model> {
    /** How many rows a row relates to. */
    readonly cardinality: "many";
    /** The related rows' model. */
    readonly target: Target;
}

/** A relational read of a model's rows, any model's by default, a one relation's unordered. */
export type FindOptions<
    Entity extends Model = Model,
    Cardinality extends "one" | "many" = "many",
    Computed extends Extras = Extras,
> = {
    /** The fields each result keeps: only those set true, or all but those set false. */
    readonly columns?: { readonly [Name in keyof Entity["value"]]?: boolean };
    /** Values computed from each row's fields and relations, read like fields. */
    readonly extras?: Computed;
    /** The condition the rows meet, over their fields, the extras and relations. */
    readonly where?: ConditionOf<Entity, Computed>;
    /** The related rows each result includes, by relation. */
    readonly with?: {
        readonly [Name in keyof Entity["relations"]]?: true | IncludeOf<Entity["relations"][Name]>;
    };
} & (Cardinality extends "many"
    ? {
          /** How the rows sort by their fields and extras, completed by their key. */
          readonly orderBy?: OrderBy<Entity["row"] & ExtraValues<Computed>>;
          /** The most rows selected, per row for a relation. */
          readonly limit?: number;
      }
    : {});

/** The schema of a relational read of any model, checking options as a read receives them. */
const findOptions: schema.Schema<FindOptions> = schema.lazy(() =>
    schema.object({
        where: Condition.schema.exactOptional(),
        orderBy: OrderBy.schema.exactOptional(),
        limit: schema.number().int().min(1).exactOptional(),
        columns: schema.record(schema.string(), schema.boolean()).exactOptional(),
        extras: schema.record(schema.string(), Expression.schema).exactOptional(),
        with: schema
            .record(schema.string(), schema.union([schema.literal(true), findOptions]))
            .exactOptional(),
    }),
);

/** A relational read of any model: its schema, and the relations it includes. */
export const FindOptions = Object.assign(findOptions, {
    /** List the relations a read includes with their reads, every row's read for `true`. */
    includes<Selected extends FindOptions>(options: {
        readonly with?: { readonly [Relation in string]?: true | Selected };
    }): [string, Selected | FindOptions][] {
        return Object.entries(options.with ?? {}).flatMap(([name, selected]) =>
            selected === undefined ? [] : [[name, selected === true ? {} : selected]],
        );
    },
});

/** The read of a relation's rows: a one relation's without order and limit. */
type IncludeOf<Relation> =
    Relation extends One<infer Target>
        ? FindOptions<Target, "one">
        : Relation extends Many<infer Target>
          ? FindOptions<Target>
          : never;

/** The condition a model's rows meet, over their fields, a read's extras and their relations. */
export type ConditionOf<Entity extends Model, Computed extends Extras = {}> = Condition<
    Entity["row"] & ExtraValues<Computed>,
    {
        readonly [Name in keyof Entity["relations"]]: RelatedConditionOf<Entity["relations"][Name]>;
    }
>;

/** The condition of a relation's rows. */
type RelatedConditionOf<Relation> = Relation extends One<infer Target> | Many<infer Target>
    ? ConditionOf<Target>
    : never;

/** One result of a relational read: the selected fields, the extras and each relation's results. */
export type FindResult<Entity extends Model, Options> = SelectionOf<Entity["value"], Options> &
    ExtrasOf<Options> &
    WithOf<Entity, Options>;

/** The fields the options' columns select. */
type SelectionOf<Value, Options> = Options extends {
    readonly columns: infer Columns extends Readonly<Record<string, boolean | undefined>>;
}
    ? true extends Columns[keyof Columns]
        ? Pick<
              Value,
              {
                  [Name in keyof Columns]: Columns[Name] extends true ? Name : never;
              }[keyof Columns] &
                  keyof Value
          >
        : Omit<
              Value,
              { [Name in keyof Columns]: Columns[Name] extends false ? Name : never }[keyof Columns]
          >
    : Value;

/** The scalar values of a read's extras, read like fields, none for extras of unknown names. */
type ExtraValues<Computed extends Extras> = string extends keyof Computed
    ? {}
    : { readonly [Name in keyof Computed]: Scalar };

/** The extras options add to each result. */
type ExtrasOf<Options> = Options extends { readonly extras: infer Computed }
    ? { readonly [Name in keyof Computed]: Scalar }
    : {};

/** The results each relation in `with` adds: its one result or null, or its results. */
type WithOf<Entity extends Model, Options> = Options extends { readonly with: infer Included }
    ? {
          readonly [
              Name in keyof Included & keyof Entity["relations"]
          ]: Entity["relations"][Name] extends One<infer Target>
              ? FindResult<Target, Included[Name] extends true ? {} : Included[Name]> | null
              : Entity["relations"][Name] extends Many<infer Target>
                ? readonly FindResult<Target, Included[Name] extends true ? {} : Included[Name]>[]
                : never;
      }
    : {};
