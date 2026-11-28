import { canonicalize, type JsonObject, type JsonValue } from "@destack/schema";
import type * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import { StateMachine } from "../field/field.ts";
import type { MethodKind } from "./method.ts";

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

/** The operations inverses and compositions use on steps. */
export const Step = {
    /** Compose two steps on one object into one, keeping the first row before and revision. */
    compose(previous: Step, next: Step): Step | undefined {
        // compose only steps on the same object
        if (!Step.same(Step.target(previous), Step.target(next))) {
            return undefined;
        }

        // take every field either input sets, the first revision, and the first row before
        const { before: _replaced, ...composed } = next;
        const { revision: _predicted, ...later } = next.input;

        return {
            ...composed,
            input: { ...previous.input, ...later },
            ...(previous.before === undefined ? {} : { before: previous.before }),
        };
    },

    /** Record a call to the step object's method of a kind. */
    call(step: Step, kind: MethodKind, input: JsonObject): sync.Call | undefined {
        const found = Object.entries(step.object.methods).find(
            ([, declared]) => declared.kind === kind,
        );

        return found === undefined ? undefined : Step.record(step, found[0], input);
    },

    /** Record a call to a named method of the step's object. */
    record(step: Step, name: string, input: JsonObject): sync.Call {
        return step.object.call(name, input);
    },

    /** Build the input identifying the step's object: the object it created, else its target. */
    target(step: Step): JsonObject {
        // read the object's identifier
        const id = step.after?.["id"] ?? step.input["id"];
        if (id === undefined) {
            throw new TypeError(`'${step.object.name}.${step.name}' has no object`);
        }

        return Step.reference(step, id);
    },

    /** Build the input identifying one object of the step's type in the scope of the call. */
    reference(step: Step, id: JsonValue): JsonObject {
        // read the scope of the call
        const { field } = step.object.route;
        const scope = field === undefined ? undefined : step.input[field];
        if (field !== undefined && scope === undefined) {
            throw new TypeError(`'${step.object.name}.${step.name}' has no scope`);
        }

        return field === undefined || scope === undefined ? { id } : { [field]: scope, id };
    },

    /** Decide whether two field values have the same JSON form. */
    same(left: JsonValue | undefined, right: JsonValue | undefined): boolean {
        return canonicalize(left ?? null) === canonicalize(right ?? null);
    },

    /** Decide whether restoring a step moves each state it changed back along a transition. */
    isReversible(step: Step, names: readonly string[]): boolean {
        return names.every((name) => {
            // pass a field that is no state, or a state the step left as it was
            const machine = step.object.fields[name]?.machine;
            const [from, to] = [step.after?.[name], step.before?.[name]];
            if (machine === undefined || Step.same(from, to)) {
                return true;
            }

            return (
                typeof from === "string" &&
                typeof to === "string" &&
                StateMachine.between(machine, from, to) !== undefined
            );
        });
    },
};
