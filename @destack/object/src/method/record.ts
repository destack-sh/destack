import type { Creation } from "@destack/access";
import { type InsertValue, Row, Scalar, Table } from "@destack/db";

import { TagMap } from "@destack/resource";
import { type JsonObject, present, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import type * as sync from "@destack/sync";
import { CallError } from "../error/call.ts";

import { MethodContext } from "./context.ts";
import { type InputObject, InputSchema } from "./input.ts";

import type { CreateOptions, Variants } from "./builder.ts";
import type { MethodVisibility } from "./method.ts";
import type { MethodProcedure, ObjectSchema } from "./procedure.ts";
import { Step } from "./step.ts";
import { Casing } from "../object/name.ts";
import type { ObjectType } from "../object/object.ts";

/** The identifiers of the objects a bulk creation created, in item order. */
export const BatchIds = schema.object({ ids: schema.array(schema.string()) });

/** One item of a bulk call: one creation's or one update's fields. */
export const BatchItem = schema.jsonObject();

/** The labels a creation or an update writes beside the fields, every record's. */
export const TagFields = { tags: TagMap.exactOptional() };

/** The field values a bulk update matches objects by, by field name. */
export const MATCH = schema.record(schema.string(), Scalar);

/** The object and revision an item of a bulk update refers to. */
export const UPDATED_ITEM = schema.looseObject({
    id: schema.string(),
    revision: schema.number().int().positive(),
});

/** The record every object has: its methods' definitions and handlers, and their checks. */
export const ObjectRecord = {
    /** Read a method's input fields. */
    fields(input: InputObject | undefined): InputObject["shape"] {
        return input?.shape ?? {};
    },

    /** Derive a creation's procedure at the collection, or by name below it for another. */
    createProcedure(
        name: string,
        schemas: ObjectSchema,
        options: Pick<
            CreateOptions<string, InputObject, schema.Schema, MethodVisibility>,
            "fields" | "visibility" | "input" | "variants" | "filled"
        >,
    ): MethodProcedure {
        // take the shared fields, the caller leaving out those the handler fills
        const written = schemas.written(options.fields, false);
        const filled = Object.fromEntries(
            (options.filled ?? []).map((column) => [
                column,
                present(written[column], `the written field ${column}`).exactOptional(),
            ]),
        );
        const shared = schemas.scope.extend({
            ...schemas.created,
            ...schemas.parent,
            ...schemas.replay,
            ...written,
            ...TagFields,
            ...filled,
            ...(options.visibility !== undefined && options.visibility !== "client"
                ? schemas.callers
                : {}),
            ...ObjectRecord.fields(options.input),
        });

        return {
            route: { method: "POST", path: name === "create" ? "" : `/${Casing.kebab(name)}` },
            input:
                options.variants === undefined
                    ? shared
                    : ObjectRecord.variantInput(name, schemas, shared, options.variants),
            output: schemas.row,
        };
    },

    /** Join each variant's columns to a creation's shared fields under its discriminating value. */
    variantInput(
        name: string,
        schemas: ObjectSchema,
        shared: InputObject,
        variants: Variants,
    ): InputSchema {
        // require one field telling the variants apart
        const [sum, ...others] = Object.entries(variants);
        if (sum === undefined || others.length > 0) {
            throw new TypeError(
                `creation '${name}' needs exactly one field to tell its variants apart`,
            );
        }

        // take each variant's columns, required and non-null
        const [field, byValue] = sum;
        const objects = Object.entries(byValue).map(([value, written]) =>
            shared.extend({ [field]: schema.literal(value), ...schemas.required(written) }),
        );

        return InputSchema.union(field, objects);
    },

    /** Prefix a failing item's call error with its index, mapping a conflict to its fields. */
    itemFailure(object: ObjectType, itemIndex: number, error: unknown): unknown {
        // pass other failures as they are
        const conflict = object.conflict(error);
        const failure = CallError.of(conflict);
        if (failure === undefined) {
            return conflict;
        }

        return new CallError(failure.code, {
            reason: failure.reason,
            domain: failure.domain,
            status: failure.status,
            message: `items.${itemIndex}: ${failure.message}`,
            data: failure.data,
            cause: failure,
        });
    },

    /** Delete each object a bulk creation created, undoable only by a type with a delete method. */
    deleteCreated(step: Step): readonly sync.Call[] | undefined {
        const { ids } = BatchIds.parse(step.result);
        const calls = ids.flatMap(
            (id) => Step.call(step, "delete", Step.reference(step, id)) ?? [],
        );

        return calls.length === 0 ? undefined : calls;
    },

    /** Restore the fields a conflicting creation changed on an existing object. */
    restoreConflict(step: Step, fields: readonly string[]): readonly sync.Call[] | undefined {
        const changed = fields.filter(
            (name) => !Step.same(step.before?.[name], step.after?.[name]),
        );

        return restoreFields(step, changed, (input) => Step.call(step, "update", input));
    },

    /** Restore the updated fields a step wrote that stayed unchanged since. */
    restoreUpdated(step: Step): readonly sync.Call[] | undefined {
        const written = step.object.written.filter((name) => Object.hasOwn(step.input, name));

        return restoreFields(step, written, (input) => Step.record(step, step.name, input));
    },

    /** Refuse tracked objects, whose change records bulk calls skip. */
    requireUnrecorded(object: ObjectType): void {
        if (object.lifecycle.tracked !== undefined) {
            throw new TypeError(
                `object ${object.name} records each change, so it cannot apply many changes at once`,
            );
        }
    },

    /** Refuse guarded fields, whose write permission bulk updates skip. */
    requireUnguarded(object: ObjectType, written: readonly string[]): void {
        const guarded = written.find((name) => object.fields[name]?.access?.write !== undefined);
        if (guarded !== undefined) {
            throw new TypeError(
                `object ${object.name} guards field '${guarded}', so it cannot apply many updates at once`,
            );
        }
    },

    /** Refuse moving numbered objects to another value they count within, which renumbers each. */
    requireUnnumbered(object: ObjectType, written: readonly string[]): void {
        const within = object.numbering?.within;
        if (within !== undefined && written.includes(within)) {
            throw new TypeError(
                `object ${object.name} numbers within '${within}', so it cannot apply many updates to it at once`,
            );
        }
    },

    /** Decode the written fields an input gives values. */
    writtenValues(table: Table, input: JsonObject, written: readonly string[]): Row {
        return Table.decode(
            table,
            Object.fromEntries(
                written.flatMap((name) => {
                    const value = input[name];

                    return value === undefined ? [] : [[name, value]];
                }),
            ),
        );
    },

    /** Build a created object's complete row from its written values and the columns' defaults. */
    createdRow(table: Table, values: InsertValue<Table>): Row {
        return Table.complete(table, Table.values(table, Row.parse(values)));
    },

    /** Relate the caller to a new object, and a new scope's owner role to its holders. */
    relateCreator(relation: string): (context: MethodContext, object: ObjectReference) => Creation {
        return (call, object) => {
            // relate the creator
            const caller = call.caller.subject;
            const isScope = call.object.policy.definition.scope === true;

            return {
                relationships: [{ relation, subject: caller }],
                ...(isScope ? { owner: { ...object, relation } } : {}),
            };
        };
    },
};

/** Restore a step's fields unchanged since through a call, none for a deleted object. */
function restoreFields(
    step: Step,
    fields: readonly string[],
    call: (input: JsonObject) => sync.Call | undefined,
): readonly sync.Call[] | undefined {
    // restore nothing for a deleted object
    const current = step.current;
    if (current === undefined) {
        return undefined;
    }

    // restore the fields unchanged since
    const restored = Object.fromEntries(
        fields
            .filter((name) => Step.same(current[name], step.after?.[name]))
            .map((name) => [name, step.before?.[name] ?? null]),
    );
    const restoring =
        Object.keys(restored).length === 0
            ? undefined
            : call({ ...Step.target(step), ...restored });

    return restoring === undefined ? undefined : [restoring];
}
