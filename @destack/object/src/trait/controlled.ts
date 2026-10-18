import { check, integer, json, text, type Check, textEnum } from "@destack/db";
import { Plan, RISKS } from "@destack/resource";
import { deletionColumns } from "./recoverable.ts";
import { derivePermissions, type Trait } from "./trait.ts";

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

/** Declare the desired generation and the controllers' observations. */
export function controlledColumns() {
    return {
        /** The desired state's generation. */
        generation: integer("generation").notNull().default(1),
        /** The conditions the controllers report, each at the generation it observed. */
        conditions: json("conditions", ConditionMap).notNull().default({}),
    };
}

/** Declare the approval threshold, the waiting plan and the accepted plan digest. */
export function approvalColumns() {
    return {
        /** The least plan risk that waits for approval, absent until the owner sets it. */
        approval: textEnum("approval", RISKS),
        /** The plan waiting for approval, absent once applied. */
        plan: json("plan", Plan),
        /** The digest of the accepted plan, applied once the controller plans it again. */
        approvedPlan: text("approved_plan"),
    };
}

/** Records a controller reconciles. */
export const controlled: Trait<ControlledDefinition> & {
    /** Require valid revisions and generations. */
    readonly checks: (name: string) => Check[];
} = {
    key: "controlled",
    storages: ["durable"],
    options: (definition) => definition.controlled,
    columns: (options) => ({
        ...controlledColumns(),
        ...deletionColumns(),
        ...(options === true ? {} : approvalColumns()),
    }),
    constraints: (_options, table) => controlled.checks(table),
    // let an object representing a principal control itself, and only roles approve its plans
    policy: (options, object, permissions) =>
        derivePermissions(
            permissions,
            ["control", ...(options === true ? [] : ["approve"])],
            object.isPrincipal ? ["control"] : [],
        ),
    methods: (options) => ({ observe, finalize, ...(options === true ? {} : { approvePlan }) }),
    checks(name: string): Check[] {
        return [
            check(`${name}_revision`, { revision: { gte: 1 } }),
            check(`${name}_generation`, { generation: { gte: 1 } }),
        ];
    },
};
