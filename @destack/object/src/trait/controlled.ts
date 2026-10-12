import { check, integer, json, text } from "@destack/db";
import { Plan, RISKS } from "@destack/resource";
import { deletionColumns } from "./recoverable.ts";
import { traitPermissions, type Trait } from "./trait.ts";

import { approvePlan, finalize, observe } from "../method/controlled.ts";
import { ConditionMap } from "../object/condition.ts";
/** How a controller reconciles records: alone, or applying risky plans only once approved. */
export type ControlledDefinition =
    | true
    | {
          /** Whether plans at the space's approval risk wait for an approver. */
          readonly approval: true;
      };

/** The methods a controlled object's controller calls. */
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
        conditions: json("conditions", ConditionMap).notNull().default({}),
    };
}

/** Declare the approval threshold, the waiting plan and the accepted plan digest. */
export function approvalColumns() {
    return {
        /** The least plan risk that waits for approval, absent until the owner sets it. */
        approval: text("approval", { enum: RISKS }),
        /** The plan waiting for approval, absent once applied. */
        plan: json("plan", Plan),
        /** The digest of the accepted plan, applied once the controller plans it again. */
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
    constraints: (_options, table) => controlledChecks(table),
    policy: (options, _object, permissions) =>
        traitPermissions(permissions, ["control", ...(options === true ? [] : ["approve"])]),
    methods: (options) => ({ observe, finalize, ...(options === true ? {} : { approvePlan }) }),
};

/** Require valid revisions and generations, and refuse observations of future generations. */
function controlledChecks(name: string) {
    return [
        check(`${name}_revision`, { revision: { gte: 1 } }),
        check(`${name}_generation`, { generation: { gte: 1 } }),
        check(`${name}_observed_generation`, {
            observedGeneration: { gte: 0, lte: { field: "generation" } },
        }),
    ];
}
