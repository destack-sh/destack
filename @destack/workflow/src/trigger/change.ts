import { schema } from "@destack/schema";
import { DeclarationReference } from "@destack/package/declare";
import { type Key, type Table, type RowImage, type Row, LogPosition, Condition } from "@destack/db";

/** The changes a change trigger fires on. */
export const CHANGE_OPERATIONS = ["create", "update", "delete"] as const;

/** A change a change trigger fires on. */
export const ChangeOperation = schema.enum(CHANGE_OPERATIONS);
/** A change a change trigger fires on. */
export type ChangeOperation = schema.Infer<typeof ChangeOperation>;

/** The changes of one object type a trigger fires on, as the manifest describes them. */
export const ChangeOn = schema.object({
    /** The changed object type. */
    object: DeclarationReference,
    /** The condition on the object's rows. */
    where: Condition.schema.exactOptional(),
    /** The operations fired on. */
    operations: schema.array(ChangeOperation).min(1),
    /** Where a new trigger starts: at the log's head, or with every matching row as created. */
    from: schema.enum(["now", "snapshot"]),
});
/** The changes of one object type a trigger fires on, as the manifest describes them. */
export type ChangeOn = schema.Infer<typeof ChangeOn>;

/** The changed object. */
export type ChangedObject<Target> = Target extends {
    readonly table: infer Definition extends Table;
}
    ? RowImage<Definition>
    : Row;

/** One change a change trigger fires on. */
export interface ObjectChange<Target = unknown> {
    /** The change's position in the log. */
    readonly position: LogPosition;
    /** The row's key, for snapshot rows. */
    readonly key?: Key;
    /** The change as the trigger sees it. */
    readonly operation: ChangeOperation;
    /** The row before an update or deletion. */
    readonly before?: ChangedObject<Target>;
    /** The row after a creation or update. */
    readonly after?: ChangedObject<Target>;
}
