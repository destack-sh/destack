import { Table } from "@destack/db";
import { ObjectError } from "../error/error.ts";
import { schema } from "@destack/schema";

import { Step } from "../method/step.ts";
import { Casing } from "../object/name.ts";
import type { StateField, StateMachine } from "../field/field.ts";
import { defineMethod, Method } from "../method/method.ts";
import {
    type Procedure,
    type ReplayFields,
    type RequiredFields,
    type RowSchema,
    type TargetFields,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

import type {} from "@destack/package/import-meta";

/** The state fields of an object and the machines they follow, by field name. */
export type StateFields = readonly (readonly [string, StateMachine])[];

/** The transition names of an object's state fields. */
type TransitionName<Fields> = {
    [Property in keyof Fields]: Fields[Property] extends StateField<infer Machine>
        ? keyof Machine["transitions"] & string
        : never;
}[keyof Fields];

/** The transition of an object's state fields under a name. */
type TransitionOf<Fields, Name> = {
    [Property in keyof Fields]: Fields[Property] extends StateField<infer Machine>
        ? Name extends keyof Machine["transitions"]
            ? Machine["transitions"][Name]
            : never
        : never;
}[keyof Fields];

/** The fields a transition writes beside its state. */
type TransitionFields<Transition> = Transition extends {
    readonly fields: readonly (infer Field extends string)[];
}
    ? Field
    : never;

/** Whether a transition stays off public routes. */
type TransitionInternal<Transition> = Transition extends { readonly isInternal: true }
    ? true
    : false;

/** The methods the transitions of an object's state fields derive. */
export type TransitionMethodMap<Fields> = {
    readonly [Name in TransitionName<Fields>]: Method<{
        kind: "transition";
        permission: string;
        mutates: true;
        fields: TransitionFields<TransitionOf<Fields, Name>>;
        internal: TransitionInternal<TransitionOf<Fields, Name>>;
    }>;
};

/** Transitions of state fields. */
export const transitions: Trait<StateFields> & {
    /** Change one state field through one transition. */
    readonly method: (
        field: string,
        name: string,
        transition: StateMachine["transitions"][string],
    ) => Method;
} = {
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
                methods[name] = transitions.method(field, name, transition);
            }
        }

        return methods;
    },
    method(field: string, name: string, transition: StateMachine["transitions"][string]): Method {
        return defineMethod<{ kind: "transition" }>({
            kind: "transition",
            permission: transition.permission,
            mutates: true,
            ...(transition.fields === undefined ? {} : { fields: transition.fields }),
            ...(transition.isInternal === true ? { isInternal: true } : {}),
            transition: { field, from: transition.from, to: transition.to },
            inverse: (step) => {
                // find the transition leading back
                const left = step.before?.[field];
                const back = Object.entries(step.object.methods).find(
                    ([, declared]) =>
                        declared.transition?.field === field &&
                        declared.transition.to === left &&
                        declared.transition.from.includes(transition.to),
                );

                return back === undefined
                    ? undefined
                    : [Step.record(step, back[0], Step.target(step))];
            },
            target: "object",
            result: "object",
            procedure: (method, schemas) => ({
                route: { method: "POST", path: `/{id}/${Casing.kebab(method)}` },
                input: schemas.target.extend({
                    ...schemas.replay,
                    ...(transition.fields === undefined ? {} : schemas.recorded(transition.fields)),
                }),
                output: schemas.row,
            }),
            handler: (context) => {
                // write the state with the fields the transition records
                const written = Object.fromEntries(
                    (transition.fields ?? []).flatMap((recorded) => {
                        const value = context.input[recorded];

                        return value === undefined ? [] : [[recorded, value]];
                    }),
                );

                return context.update(
                    Table.values(context.object.table, {
                        ...Table.decode(context.object.table, written),
                        [field]: transition.to,
                    }),
                );
            },
            async execute(context, execution) {
                // refuse transitions from any other state
                const state = schema.string().parse(context.target[field]);
                if (!transition.from.includes(state)) {
                    throw new ObjectError("INVALID_TRANSITION", {
                        message: `${context.object.name} cannot ${name} while ${field} is ${state}`,
                    });
                }

                return Method.handle(this, context, execution);
            },
        });
    },
};

/** The procedures transitions derive, taking the fields a transition records. */
export type TransitionProcedures<Object extends ObjectType, Declared extends Method = Method> = {
    transition: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> &
                RequiredFields<Object, NonNullable<Declared["fields"]>[number] & string>
        >,
        RowSchema<Object>
    >;
};
