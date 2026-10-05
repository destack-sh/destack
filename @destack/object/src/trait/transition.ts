import { schema } from "@destack/schema";
import type { StateField, StateMachine } from "../field/field.ts";
import { type Method } from "../method/method.ts";
import {
    type Procedure,
    type ReplayShape,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

import { transitionMethod } from "../method/transition.ts";
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
    readonly [Name in TransitionName<Fields>]: Method<{
        kind: "transition";
        permission: string;
        mutates: true;
    }>;
};

/** Transitions of state fields. */
export const transitions: Trait<StateFields> = {
    options: (definition) => {
        // collect the fields following a machine
        const fields = Object.entries(definition.fields ?? {}).flatMap(([name, declared]) =>
            declared.machine === undefined ? [] : [[name, declared.machine] as const],
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

/** The procedures transitions derive. */
export type TransitionProcedures<Object extends ObjectType> = {
    transition: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object>>,
        RowSchema<Object>
    >;
};
