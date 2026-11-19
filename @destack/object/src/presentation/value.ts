import { type JsonValue, schema } from "@destack/schema";
import { type FieldType, StateMachine, type StateStage } from "../field/field.ts";
import type { ObjectType } from "../object/object.ts";

/** A field's value with what it means for drawing it, by its field's type. */
export type ObjectValue =
    | { readonly kind: "absent" }
    | { readonly kind: "text"; readonly value: string }
    | { readonly kind: "number"; readonly value: number }
    | { readonly kind: "boolean"; readonly value: boolean }
    | { readonly kind: "time"; readonly value: number }
    | { readonly kind: "choice"; readonly value: string }
    | { readonly kind: "state"; readonly value: string; readonly stage: StateStage }
    | { readonly kind: "reference"; readonly value: string; readonly target: string }
    | { readonly kind: "json"; readonly value: JsonValue };

/** The kind of value each field type stores, besides absence. */
const KINDS: Readonly<Record<FieldType, Exclude<ObjectValue["kind"], "absent">>> = {
    string: "text",
    text: "text",
    bigint: "text",
    fractionalIndex: "text",
    integer: "number",
    number: "number",
    boolean: "boolean",
    time: "time",
    enum: "choice",
    state: "state",
    reference: "reference",
    binary: "json",
    json: "json",
    message: "json",
    subject: "json",
    object: "json",
    file: "json",
    blob: "json",
    blobs: "json",
    blobMap: "json",
};

/** A field's value with what it means for drawing it. */
export const ObjectValue = {
    /** Read a field's stored value by its field's type, refusing a value its type cannot store. */
    of(type: ObjectType, name: string, value: JsonValue | undefined): ObjectValue {
        // read absence, a column outside the fields such as the identifier as text, then by kind
        const field = type.fields[name];
        if (value === undefined || value === null) {
            return { kind: "absent" };
        } else if (field === undefined) {
            return { kind: "text", value: schema.string().parse(value) };
        }

        // read the value by its field's kind
        const kind = KINDS[field.type];
        const machine = field.machine;
        const target = field.target?.().name;
        if (kind === "number" || kind === "time") {
            return { kind, value: schema.number().parse(value) };
        } else if (kind === "boolean") {
            return { kind, value: schema.boolean().parse(value) };
        } else if (kind === "state" && machine !== undefined) {
            const state = schema.string().parse(value);

            return { kind, value: state, stage: StateMachine.stage(machine, state) };
        } else if (kind === "reference" && target !== undefined) {
            return { kind, value: schema.string().parse(value), target };
        } else if (kind === "choice") {
            return { kind, value: schema.string().parse(value) };
        } else if (kind === "text" || kind === "reference") {
            return { kind: "text", value: schema.string().parse(value) };
        }

        return { kind: "json", value };
    },
};
