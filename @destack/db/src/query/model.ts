import type { Scalar, Condition } from "./condition.ts";
import type { Extras } from "./namespace.ts";
import type { OrderBy } from "./order.ts";
import type { Aggregate } from "./option.ts";

/** What relational reads see of a table or object type: the row conditions compare, the value results hold, and its relations. */
export interface Model {
    /** The fields conditions and orders compare. */
    readonly row: object;
    /** The fields each result holds. */
    readonly value: object;
    /** The relations, by name. */
    readonly relations: object;
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

/** A relational read of a model's rows, a one relation's without order and limit. */
export type FindOptions<
    Entity extends Model,
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
        readonly [Name in keyof Entity["relations"]]?:
            | true
            | (Entity["relations"][Name] extends One<infer Target>
                  ? FindOptions<Target, "one">
                  : Entity["relations"][Name] extends Many<infer Target>
                    ? FindOptions<Target>
                    : never);
    };
} & (Cardinality extends "many"
    ? {
          /** How the rows sort by their fields and extras, completed by their key. */
          readonly orderBy?: OrderBy<Entity["row"] & ExtraValues<Computed>>;
          /** The most rows selected, per row for a relation. */
          readonly limit?: number;
      }
    : {});

/** Measures of a model's rows per group. */
export type AggregateOptions<Entity extends Model> = {
    /** The condition the measured rows meet. */
    readonly where?: ConditionOf<Entity>;
} & Aggregate;

/** The condition a model's rows meet, over their fields, a read's extras and their relations. */
export type ConditionOf<Entity extends Model, Computed extends Extras = {}> = Condition<
    Entity["row"] & ExtraValues<Computed>,
    {
        readonly [Name in keyof Entity["relations"]]: Entity["relations"][Name] extends
            | One<infer Target>
            | Many<infer Target>
            ? ConditionOf<Target>
            : never;
    }
>;

/** One result of a relational read: the selected fields, the extras and each relation's results. */
export type FindResult<Entity extends Model, Options> = Selected<Entity["value"], Options> &
    ExtrasOf<Options> &
    WithOf<Entity, Options>;

/** One group of an aggregate read: its group values and its measures. */
export type Group<Options extends Aggregate = Aggregate> = {
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

/** The fields the options' columns select. */
type Selected<Value, Options> = Options extends {
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

/** The results each relation of the options' `with` adds: its one result or null, or its results. */
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

/** The group columns of an aggregate. */
type GroupColumnOf<Options extends Aggregate> = Options extends {
    readonly groupBy: readonly (infer Column extends string)[];
}
    ? Column
    : never;
