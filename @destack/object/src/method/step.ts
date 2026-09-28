import { canonicalize } from "@destack/schema/json";
import type * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { Method, MethodKind } from "./method.ts";
import { Call } from "./call.ts";

/** A row of an object as a client holds it, by field. */
type Row = Readonly<Record<string, unknown>>;

/** A call that happened, with the target's rows before, after and now. */
export interface Step {
    /** The object type the call acted on. */
    readonly object: ObjectType;
    /** The method's name. */
    readonly name: string;
    /** The call's input, its routing fields included. */
    readonly input: Readonly<Record<string, unknown>>;
    /** What the call returned. */
    readonly result?: unknown;
    /** The object's row before the call. */
    readonly before?: Row;
    /** The object's row after the call. */
    readonly after?: Row;
    /** The object's row as the client holds it now. */
    readonly current?: Row;
}

/** The operations inverses use on steps. */
export const Step = {
    /** Record a call to the step object's method of a kind. */
    call(
        step: Step,
        kind: MethodKind,
        input: Readonly<Record<string, unknown>>,
    ): sync.Call | undefined {
        const found = Object.entries(step.object.methods as Readonly<Record<string, Method>>).find(
            ([, declared]) => declared.kind === kind,
        );

        return found === undefined ? undefined : Step.record(step, found[0], input);
    },

    /** Record a call to a named method of the step's object. */
    record(step: Step, name: string, input: Readonly<Record<string, unknown>>): sync.Call {
        return Call.record(step.object, name, input);
    },

    /** Build the input naming the step's object. */
    target(step: Step, id: unknown): Record<string, unknown> {
        const { field } = step.object.route;

        return field === undefined ? { id } : { [field]: step.input[field], id };
    },

    /** Decide whether two field values have the same JSON form. */
    same(left: unknown, right: unknown): boolean {
        return canonicalize(left ?? null) === canonicalize(right ?? null);
    },
};
