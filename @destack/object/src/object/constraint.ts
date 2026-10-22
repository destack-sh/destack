import {
    check,
    foreignKey,
    index,
    TABLE,
    unique,
    uniqueIndex,
    type Column,
    type Condition,
    type ReferenceAction,
    type Table,
    type TableConstraint,
} from "@destack/db";
import { Casing } from "./name.ts";
import type { ObjectType } from "./object.ts";

/** A constraint over an object's columns by property, which a definition keys by name. */
export type ObjectConstraint<Property extends string = string, Foreign extends string = string> =
    | {
          /** A condition every object meets. */
          readonly kind: "check";
          /** The condition. */
          readonly condition: Condition;
      }
    | {
          /** An ordered index. */
          readonly kind: "index";
          /** The indexed columns, in key order. */
          readonly on: readonly Property[];
          /** Whether each key identifies at most one object. */
          readonly unique?: true;
          /** The condition selecting the indexed objects, all objects when absent. */
          readonly where?: Condition;
          /** Whether the unique index treats nulls as equal values. */
          readonly nullsNotDistinct?: true;
      }
    | {
          /** Columns whose non-null values are distinct together. */
          readonly kind: "unique";
          /** The constrained columns. */
          readonly on: readonly Property[];
      }
    | (ReferenceAction & {
          /** A reference from some columns to those of another table. */
          readonly kind: "foreignKey";
          /** The referencing columns, in order. */
          readonly on: readonly Property[];
          /** The referenced object type, or the objects' type, and its columns in order. */
          readonly references: {
              /** The referenced object type, or "self" for the objects' type. */
              readonly object: (() => ObjectType) | "self";
              /** The referenced columns, in matching order. */
              readonly on: readonly Foreign[];
          };
      });

/** The constraints a definition declares. */
export const ObjectConstraint = {
    /** Build a declared constraint over an object's table, named after the table. */
    build(name: string, constraint: ObjectConstraint, table: Table): TableConstraint {
        const sqlName = `${table[TABLE].name}_${Casing.snake(name)}`;
        switch (constraint.kind) {
            case "check":
                return check(sqlName, constraint.condition);
            case "index": {
                const selected = (constraint.unique ? uniqueIndex : index)(sqlName).on(
                    ...ObjectConstraint.readColumns(constraint.on, table),
                );
                const filtered =
                    constraint.where === undefined ? selected : selected.where(constraint.where);

                return constraint.nullsNotDistinct ? filtered.nullsNotDistinct() : filtered;
            }
            case "unique":
                return unique(sqlName).on(...ObjectConstraint.readColumns(constraint.on, table));
            case "foreignKey": {
                const { object, on } = constraint.references;
                const referenced = object === "self" ? table : object().table;
                const built = foreignKey({
                    name: sqlName,
                    columns: ObjectConstraint.readColumns(constraint.on, table),
                    foreignColumns: ObjectConstraint.readColumns(on, referenced),
                });
                const deleted =
                    constraint.onDelete === undefined ? built : built.onDelete(constraint.onDelete);

                return constraint.onUpdate === undefined
                    ? deleted
                    : deleted.onUpdate(constraint.onUpdate);
            }
        }
    },

    /** Read a table's columns by property, in order, refusing a constraint without columns. */
    readColumns(properties: readonly string[], table: Table): [Column, ...Column[]] {
        // refuse a constraint without columns
        const [first, ...rest] = properties;
        const definition = table[TABLE];
        if (first === undefined) {
            throw new TypeError(`a constraint on ${definition.name} has no column`);
        }

        return [definition.column(first), ...rest.map((property) => definition.column(property))];
    },
};
