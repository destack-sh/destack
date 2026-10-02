import type { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Field, StateField, StateMachine } from "../field/field.ts";
import { Step } from "../method/step.ts";
import { defineMethod, type Method } from "../method/method.ts";
import {
    type Procedure,
    type ReplayShape,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import { kebabCase } from "../object/name.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** The state fields of an object and the machines they follow, by field name. */
export type StateFields = readonly (readonly [string, StateMachine])[];

/** The transition names of an object's state fields. */
type TransitionName<Fields> = {
    [Property in keyof Fields]: Fields[Property] extends StateField<infer Machine>
        ? keyof Machine["transitions"] & string
        : never;
}[keyof Fields];

/** The methods the transitions of an object's state fields derive. */
export type TransitionMethodMap<Fields> = {
    readonly [Name in TransitionName<Fields>]: Method<"transition", string, never, never, true>;
};

/** Transitions of state fields. */
export const transitions: Trait<StateFields> = {
    options: (definition) => {
        // collect the fields following a machine
        const fields = Object.entries(definition.fields ?? {}).flatMap(([name, declared]) =>
            (declared as Field).machine === undefined
                ? []
                : [[name, (declared as Field).machine!] as const],
        );

        return fields.length === 0 ? undefined : fields;
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (fields) => {
        // derive one method per transition
        const methods: Record<string, Method> = {};
        for (const [field, machine] of fields) {
            for (const [name, transition] of Object.entries(machine.transitions)) {
                if (Object.hasOwn(methods, name)) {
                    throw new TypeError(
                        `transition ${name} is declared by more than one state field`,
                    );
                }
                methods[name] = transitionMethod(field, name, transition);
            }
        }

        return methods;
    },
};

/** Change one state field through one transition. */
function transitionMethod(
    field: string,
    name: string,
    transition: StateMachine["transitions"][string],
): Method {
    return defineMethod<Method<"transition">>({
        kind: "transition",
        permission: transition.permission,
        mutates: true,
        transition: { field, from: transition.from, to: transition.to },
        inverse: (step) => {
            // find the transition leading back
            const left = step.before?.[field];
            const back = Object.entries(
                step.object.methods as Readonly<Record<string, Method>>,
            ).find(
                ([, declared]) =>
                    declared.transition?.field === field &&
                    declared.transition.to === left &&
                    declared.transition.from.includes(transition.to),
            );

            return back === undefined
                ? undefined
                : [Step.record(step, back[0], Step.target(step, step.input.id))];
        },
        target: true,
        result: "object",
        procedure: (method, shapes) => ({
            route: { method: "POST", path: `/{id}/${kebabCase(method)}` },
            input: shapes.target.extend(shapes.replay),
            output: shapes.row,
        }),
        effect: (call) => call.update({ [field]: transition.to }),
        async execute(call) {
            // refuse transitions from any other state
            const state = (call.target as Record<string, unknown>)[field] as string;
            if (!transition.from.includes(state)) {
                throw new ServiceError("CONFLICT", {
                    message: `${call.object.name} cannot ${name} while ${field} is ${state}`,
                });
            }

            return this.effect(call);
        },
    });
}

/** The procedures transitions derive. */
export type TransitionProcedures<Object extends ObjectType> = {
    transition: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object>>,
        RowSchema<Object>
    >;
};
