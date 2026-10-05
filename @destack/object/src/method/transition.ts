import { TABLE } from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { StateMachine } from "../field/field.ts";
import { Step } from "./step.ts";
import { defineMethod, type Method } from "./method.ts";
import type { Call } from "./call.ts";
import type { ObjectTable } from "../object/table.ts";
import { kebabCase } from "../object/name.ts";

/** Change one state field through one transition. */
export function transitionMethod(
    field: string,
    name: string,
    transition: StateMachine["transitions"][string],
): Method {
    return defineMethod<{ kind: "transition" }>({
        kind: "transition",
        permission: transition.permission,
        mutates: true,
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

            return back === undefined ? undefined : [Step.record(step, back[0], Step.target(step))];
        },
        target: true,
        result: "object",
        procedure: (method, shapes) => ({
            route: { method: "POST", path: `/{id}/${kebabCase(method)}` },
            input: shapes.target.extend(shapes.replay),
            output: shapes.row,
        }),
        handler: (call: Call<ObjectTable>) =>
            call.update(call.object.table[TABLE].values({ [field]: transition.to })),
        async execute(call) {
            // refuse transitions from any other state
            const state = schema.string().parse(call.requireTarget()[field]);
            if (!transition.from.includes(state)) {
                throw new ServiceError("CONFLICT", {
                    message: `${call.object.name} cannot ${name} while ${field} is ${state}`,
                });
            }

            return this.handler(call);
        },
    });
}
