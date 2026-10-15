import { Table } from "@destack/db";
import { CallError } from "@destack/runtime/error";
import { Digest, schema } from "@destack/schema";

import { MethodBuilder } from "./record.ts";

import { Observation, StatusCondition } from "../object/condition.ts";

import type { TraitTable } from "../object/table.ts";
/** The method declarations of controlled objects. */
export const controlledMethod = new MethodBuilder<TraitTable<{ readonly controlled: true }>>();

/** The method declarations of controlled objects whose risky plans wait for approval. */
export const approvalMethod = new MethodBuilder<
    TraitTable<{ readonly controlled: { readonly approval: true } }>
>();

/** Record a controller's observation of its target at its generation. */
export const observe = controlledMethod
    .mutation({ permission: "control", input: Observation })
    .handle(async (call) => {
        // merge the conditions and keep transition times while their status stays
        const { observedGeneration, conditions, fields } = call.input;
        const current = call.target.conditions;
        const merged = Object.fromEntries(
            Object.entries(conditions).map(([name, condition]) => [
                name,
                StatusCondition.observe(
                    current[name],
                    { ...condition, observedGeneration },
                    call.now,
                ),
            ]),
        );

        // write the observed state
        return call.updateStatus({
            ...Table.decode(call.object.table, fields ?? {}),
            observedGeneration,
            conditions: { ...current, ...merged },
        });
    });

/** Remove a record with a requested deletion after its controller finishes. */
export const finalize = controlledMethod
    .mutation({
        permission: "control",
        output: schema.object({}),
    })
    .handle(async (call) => {
        // require a requested deletion
        if (call.target.deletedAt === null) {
            throw new CallError("CONFLICT", {
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
        permission: "approve",
        isInternal: true,
        input: schema.object({
            /** The digest of the approved plan. */
            plan: Digest,
        }),
    })
    .handle((call) => call.update({ approvedPlan: call.input.plan }));
