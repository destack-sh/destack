import { canonicalize, type JsonObject, type JsonValue } from "@destack/schema";
import type * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { MethodKind } from "./method.ts";
import { Call } from "./call.ts";

/** A call that happened, with the target's rows before, after and now in their JSON form. */
export interface Step {
    /** The object type the call acted on. */
    readonly object: ObjectType;
    /** The method's name. */
    readonly name: string;
    /** The call's input, its routing fields included. */
    readonly input: JsonObject;
    /** What the call returned. */
    readonly result?: unknown;
    /** The object's row before the call. */
    readonly before?: JsonObject;
    /** The object's row after the call. */
    readonly after?: JsonObject;
    /** The object's row as the client has it now. */
    readonly current?: JsonObject;
}

/** The operations inverses use on steps. */
export const Step = {
    /** Record a call to the step object's method of a kind. */
    call(step: Step, kind: MethodKind, input: JsonObject): sync.Call | undefined {
        const found = Object.entries(step.object.methods).find(
            ([, declared]) => declared.kind === kind,
        );

        return found === undefined ? undefined : Step.record(step, found[0], input);
    },

    /** Record a call to a named method of the step's object. */
    record(step: Step, name: string, input: JsonObject): sync.Call {
        return Call.record(step.object, name, input);
    },

    /** Build the input naming the step's object: the object it created, else its target. */
    target(step: Step): JsonObject {
        // read the object's identifier and the scope the call named
        const id = step.after?.["id"] ?? step.input["id"];
        const { field } = step.object.route;
        const scope = field === undefined ? undefined : step.input[field];
        if (id === undefined || (field !== undefined && scope === undefined)) {
            throw new TypeError(`${step.object.name}.${step.name} names no object`);
        }

        return field === undefined || scope === undefined ? { id } : { [field]: scope, id };
    },

    /** Decide whether two field values have the same JSON form. */
    same(left: JsonValue | undefined, right: JsonValue | undefined): boolean {
        return canonicalize(left ?? null) === canonicalize(right ?? null);
    },
};
