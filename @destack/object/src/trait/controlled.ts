import { check, integer, json, Table, text, type Check, textEnum } from "@destack/db";
import { Plan, RISKS } from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { t } from "@destack/locale";
import { Recoverable } from "./recoverable.ts";
import { derivePermissions, type TraitBehavior, callableTrait } from "./trait.ts";
import { ObjectError } from "../error/error.ts";
import { MethodBuilder } from "../method/builder.ts";
import { Condition, Observation, Status, SYNCED } from "../object/condition.ts";
import type { TraitTable } from "../object/table.ts";

/** The method declarations of controlled objects. */
const controlledMethod = new MethodBuilder<TraitTable<{ readonly controlled: true }>>();

/** The method declarations of controlled objects whose risky plans wait for approval. */
const approvalMethod = new MethodBuilder<
    TraitTable<{ readonly controlled: { readonly approval: true } }>
>();

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
          readonly observe: typeof Controlled.observe;
          readonly finalize: typeof Controlled.finalize;
          readonly approvePlan: typeof Controlled.approvePlan;
      }
    : Controlled extends true
      ? {
            readonly observe: typeof Controlled.observe;
            readonly finalize: typeof Controlled.finalize;
        }
      : {};

/** Records a controller reconciles. */
const controlledTrait: TraitBehavior<ControlledDefinition> = {
    key: "controlled",
    storages: ["durable", "local"],
    options: (definition) => definition.controlled,
    columns: (options) => ({
        ...Controlled.columns(),
        ...Recoverable.columns(),
        ...(options === true ? {} : Controlled.approvalColumns()),
    }),
    constraints: (_options, table) => Controlled.checks(table),
    // let an object representing a principal control itself, and only roles approve its plans
    policy: (options, object, permissions) =>
        derivePermissions(
            permissions,
            ["control", ...(options === true ? [] : ["approve"])],
            object.isPrincipal ? ["control"] : [],
        ),
    methods: (options) => ({
        observe: Controlled.observe,
        finalize: Controlled.finalize,
        ...(options === true ? {} : { approvePlan: Controlled.approvePlan }),
    }),
};

/** Reconcile records with a controller. */
export const controlled = callableTrait("controlled", controlledTrait, true);

/** The columns, checks and methods of records a controller reconciles. */
export const Controlled = {
    /** Declare the desired generation and the controllers' observations. */
    columns: controlledColumns,

    /** Declare the approval threshold, the waiting plan and the accepted plan digest. */
    approvalColumns: approvalColumns,

    /** Require valid revisions and generations. */
    checks(name: string): Check[] {
        return [
            check(`${name}_revision`, { revision: { gte: 1 } }),
            check(`${name}_generation`, { generation: { gte: 1 } }),
        ];
    },

    /** Record a controller's conditions of its target at the generation it evaluated. */
    observe: controlledMethod
        .mutation({
            title: t.context("action")`Observe`,
            permission: "control",
            input: Observation.extend({
                /** The desired generation the controller evaluated. */
                observedGeneration: schema.number().int().min(0),
            }),
        })
        .handle(async (context) => {
            // refuse an observation of a generation the record has not arrived at
            const { observedGeneration, conditions, fields } = context.input;
            if (observedGeneration > context.target.generation) {
                throw ObjectError.of(
                    "GENERATION_CONFLICT",
                    `cannot reconcile ${context.object.name} at generation '${observedGeneration}': it is at '${context.target.generation}'`,
                );
            }

            // merge each condition observed no earlier, keeping its transition time while its status stays
            const current = context.target.conditions;
            const merged = Object.fromEntries(
                Object.entries(conditions)
                    .filter(
                        ([name]) => (current[name]?.observedGeneration ?? 0) <= observedGeneration,
                    )
                    .map(([name, condition]) => [
                        name,
                        Condition.observe(
                            current[name],
                            { ...condition, observedGeneration },
                            context.now,
                        ),
                    ]),
            );

            // write the observed state, advancing the synced generation on a synced observation
            const isSynced = conditions[SYNCED]?.status === "true";
            const { syncedGeneration } = context.target;

            return context.updateStatus({
                ...Table.decode(context.object.table, fields ?? {}),
                conditions: { ...current, ...merged },
                syncedGeneration: isSynced
                    ? Math.max(syncedGeneration, observedGeneration)
                    : syncedGeneration,
            });
        }),

    /** Remove a record with a requested deletion after its controller finishes. */
    finalize: controlledMethod
        .mutation({
            title: t.context("action")`Finalize`,
            permission: "control",
            output: schema.object({}),
        })
        .handle(async (context) => {
            // require a requested deletion
            if (context.target.deletedAt === null) {
                throw ObjectError.of(
                    "DELETION_BLOCKED",
                    `cannot finish deleting ${context.object.name}: it is not being deleted`,
                );
            }

            // delete at the loaded revision
            await context.remove();

            return {};
        }),

    /** Accept a plan digest for the controller to apply once it plans the same steps again. */
    approvePlan: approvalMethod
        .mutation({
            title: t.context("action")`Approve plan`,
            permission: "approve",
            visibility: "installation",
            input: schema.object({
                /** The digest of the approved plan. */
                plan: Digest,
            }),
        })
        .handle((context) => context.update({ approvedPlan: context.input.plan })),
};

/** Declare the desired generation and the controllers' observations. */
function controlledColumns() {
    return {
        /** The desired state's generation. */
        generation: integer("generation").notNull().default(1),
        /** The conditions the controllers report, each at the generation it observed. */
        conditions: json("conditions", Status.conditions).notNull().default({}),
        /** The latest desired generation a controller synced, zero before its first sync. */
        syncedGeneration: integer("synced_generation").notNull().default(0),
    };
}

/** The columns of a record a controller reconciles. */
export type ControlledColumns = ReturnType<typeof controlledColumns>;

/** Declare the approval threshold, the waiting plan and the accepted plan digest. */
function approvalColumns() {
    return {
        /** The least plan risk that waits for approval, absent until the owner sets it. */
        approval: textEnum("approval", RISKS),
        /** The plan waiting for approval, absent once applied. */
        plan: json("plan", Plan),
        /** The digest of the accepted plan, applied once the controller plans it again. */
        approvedPlan: text("approved_plan"),
    };
}

/** The columns of a controlled record whose risky plans wait for approval. */
export type ApprovalColumns = ReturnType<typeof approvalColumns>;
