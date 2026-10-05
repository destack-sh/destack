import { TABLE } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { Digest, schema } from "@destack/schema";
import { method, type MethodBuilder } from "./method.ts";

import { Observation, observeCondition } from "../object/condition.ts";

import type { TraitTable } from "../object/table.ts";
/** The method declarations of controlled objects. */
export const controlledMethod: MethodBuilder<TraitTable<{ readonly controlled: true }>> = method;

/** The method declarations of controlled objects whose risky plans wait for approval. */
export const approvalMethod: MethodBuilder<
    TraitTable<{ readonly controlled: { readonly approval: true } }>
> = method;

/** Record a controller's observation of its target at its generation. */
export const observe = controlledMethod
    .mutation({ permission: null, isSystem: true, input: Observation })
    .handle(async (call) => {
        // merge the conditions and keep transition times while their status stays
        const { observedGeneration, conditions, fields } = call.input;
        const current = call.target.conditions;
        const merged = Object.fromEntries(
            Object.entries(conditions).map(([name, condition]) => [
                name,
                observeCondition(current[name], { ...condition, observedGeneration }, call.now),
            ]),
        );

        // write the observed state
        return call.updateStatus({
            ...call.object.table[TABLE].decode(fields ?? {}),
            observedGeneration,
            conditions: { ...current, ...merged },
        });
    });

/** Remove a record with a requested deletion after its controller finishes. */
export const finalize = controlledMethod
    .mutation({
        permission: null,
        isSystem: true,
        output: schema.object({}),
    })
    .handle(async (call) => {
        // require a requested deletion
        if (call.target.deletionRequestedAt === null) {
            throw new ServiceError("CONFLICT", {
                message: `${call.object.name} is not being deleted`,
            });
        }

        // delete at the loaded revision
        await call.remove();

        return {};
    });

/** Accept a plan digest for the controller to apply once it plans the same steps again. */
export const approvePlan = approvalMethod
    .mutation({
        permission: null,
        isSystem: true,
        input: schema.object({
            /** The digest of the approved plan. */
            plan: Digest,
        }),
    })
    .handle((call) => call.update({ approvedPlan: call.input.plan }));
