import { Table } from "@destack/db";
import { ObjectError } from "../error/error.ts";

import { Digest, schema } from "@destack/schema";

import { MethodBuilder } from "./record.ts";

import { Observation, StatusCondition } from "../object/condition.ts";

import type { TraitTable } from "../object/table.ts";

import type {} from "@destack/package/import-meta";
/** The method declarations of controlled objects. */
export const controlledMethod = new MethodBuilder<TraitTable<{ readonly controlled: true }>>();

/** The method declarations of controlled objects whose risky plans wait for approval. */
export const approvalMethod = new MethodBuilder<
    TraitTable<{ readonly controlled: { readonly approval: true } }>
>();

/** Record a controller's conditions of its target at the generation it evaluated. */
export const observe = controlledMethod
    .mutation({ permission: "control", input: Observation })
    .handle(async (context) => {
        // refuse an observation of a generation the record has not arrived at
        const { observedGeneration, conditions, fields } = context.input;
        if (observedGeneration > context.target.generation) {
            throw ObjectError.of(
                "GENERATION_STALE",
                `expected ${context.object.name} at generation ${observedGeneration}, got ${context.target.generation}`,
            );
        }

        // merge the conditions and preserve transition times while their status stays
        const current = context.target.conditions;
        const merged = Object.fromEntries(
            Object.entries(conditions).map(([name, condition]) => [
                name,
                StatusCondition.observe(
                    current[name],
                    { ...condition, observedGeneration },
                    context.now,
                ),
            ]),
        );

        // write the observed state
        return context.updateStatus({
            ...Table.decode(context.object.table, fields ?? {}),
            conditions: { ...current, ...merged },
        });
    });

/** Remove a record with a requested deletion after its controller finishes. */
export const finalize = controlledMethod
    .mutation({
        permission: "control",
        output: schema.object({}),
    })
    .handle(async (context) => {
        // require a requested deletion
        if (context.target.deletedAt === null) {
            throw ObjectError.of(
                "DELETION_UNSTARTED",
                `${context.object.name} is not being deleted`,
            );
        }

        // delete at the loaded revision
        await context.remove();

        return {};
    });

/** Accept a plan digest for the controller to apply once it plans the same steps again. */
export const approvePlan = approvalMethod
    .mutation({
        permission: "approve",
        visibility: "package",
        input: schema.object({
            /** The digest of the approved plan. */
            plan: Digest,
        }),
    })
    .handle((context) => context.update({ approvedPlan: context.input.plan }));
