import { present, schema } from "@destack/schema";
import { Step } from "./step.ts";
import { defineMethod, type Method } from "./method.ts";
import { SequenceEdit } from "../sequence/index.ts";
import { ServiceError } from "@destack/service/error";
import { Chunk, Undo } from "../text/chunk.ts";

/** Edit one text field of an object. */
export function edit(
    required: string,
    fields: readonly [string, ...string[]],
): Method<{ kind: "edit"; permission: string; output: typeof Undo; mutates: true }> {
    return defineMethod<{ kind: "edit"; permission: string; output: typeof Undo; mutates: true }>({
        kind: "edit",
        permission: required,
        mutates: true,
        target: true,
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/edit" },
            input: shapes.target.extend({
                ...shapes.replay,
                field: schema.enum(fields),
                edits: schema.array(SequenceEdit).min(1),
            }),
            output: Undo,
        }),
        handler: (call) => Chunk.edit(call),
        inverse: (step) => {
            // apply the recorded inverse edits
            if (step.result === undefined) {
                return undefined;
            }
            const result = Undo.parse(step.result);

            return result.inverse.length === 0
                ? []
                : [
                      Step.record(step, step.name, {
                          ...Step.target(step),
                          field: schema.string().parse(step.input["field"]),
                          edits: result.inverse,
                      }),
                  ];
        },
        async execute(call) {
            // require the field's write permission when it guards writes
            const field = schema.string().parse(call.input["field"]);
            const guard = present(
                call.object.fields[field],
                `the text field ${field} of ${call.object.name}`,
            ).access?.write;
            if (!call.isPredicted && guard !== undefined) {
                const decision = await call
                    .requireAuthorization()
                    .check(call.object.permission(guard), call.reference());
                if (!decision.isAllowed) {
                    throw new ServiceError("FORBIDDEN", {
                        message: `field ${field} is not writable`,
                    });
                }
            }

            return this.handler(call);
        },
    });
}
