import { check, integer, json, sql, text, type Column } from "@destack/db";
import { Plan, RISKS } from "@destack/resource";
import { present } from "@destack/schema";
import { deletionColumns } from "./recoverable.ts";
import type { Trait } from "./trait.ts";

import { approvePlan, finalize, observe } from "../method/controlled.ts";
import { ConditionMap } from "../object/condition.ts";
/** How a controller reconciles records: alone, or applying risky plans only once approved. */
export type ControlledDefinition =
    | true
    | {
          /** Whether plans reaching the space's approval risk wait for an approver to accept them. */
          readonly approval: true;
      };

/** The system methods a controlled object's controller calls. */
export type ControlledMethodMap<Controlled> = Controlled extends { readonly approval: true }
    ? {
          readonly observe: typeof observe;
          readonly finalize: typeof finalize;
          readonly approvePlan: typeof approvePlan;
      }
    : Controlled extends true
      ? { readonly observe: typeof observe; readonly finalize: typeof finalize }
      : {};

/** Declare the desired generation and a controller's observations. */
export function controlledColumns() {
    return {
        /** The desired state's generation. */
        generation: integer("generation").notNull().default(1),
        /** The latest generation the controller evaluated. */
        observedGeneration: integer("observed_generation").notNull().default(0),
        /** The conditions the controller reports. */
        conditions: json("conditions", ConditionMap)
            .notNull()
            .default(sql`'{}'`),
    };
}

/** Declare the approval threshold, the plan waiting for approval and the digest an approver accepted. */
export function approvalColumns() {
    return {
        /** The least risk of a plan that waits for approval, absent until the record's owner sets it. */
        approval: text("approval", { enum: RISKS }),
        /** The plan waiting for approval, absent once applied. */
        plan: json("plan", Plan),
        /** The digest of the plan an approver accepted, applied once the controller plans it again. */
        approvedPlan: text("approved_plan"),
    };
}

/** Records a controller reconciles. */
export const controlled: Trait<ControlledDefinition> = {
    key: "controlled",
    isDurable: true,
    options: (definition) => definition.controlled,
    columns: (options) => ({
        ...controlledColumns(),
        ...deletionColumns(),
        ...(options === true ? {} : approvalColumns()),
    }),
    constraints: (_options, table, columns) =>
        controlledChecks(table, {
            revision: present(columns["revision"], "the revision column"),
            generation: present(columns["generation"], "the generation column"),
            observedGeneration: present(columns["observedGeneration"], "the observed column"),
        }),
    methods: (options) => ({ observe, finalize, ...(options === true ? {} : { approvePlan }) }),
};

/** Require valid revisions and generations, and refuse observations of future generations. */
function controlledChecks(
    name: string,
    columns: {
        /** The record revision, including observation changes. */
        readonly revision: Column;
        /** The desired generation. */
        readonly generation: Column;
        /** The observed generation. */
        readonly observedGeneration: Column;
    },
) {
    return [
        check(`${name}_revision`, sql`${columns.revision} >= 1`),
        check(`${name}_generation`, sql`${columns.generation} >= 1`),
        check(
            `${name}_observed_generation`,
            sql`${columns.observedGeneration} BETWEEN 0 AND ${columns.generation}`,
        ),
    ];
}
