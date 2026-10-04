import { check, integer, json, sql, text, type Column, TABLE } from "@destack/db";
import { Plan } from "@destack/resource";
import { ServiceError } from "@destack/service/error";
import { defineSchema, Digest, Instant, present, schema } from "@destack/schema";
import { method, type MethodBuilder } from "../method/method.ts";
import type { ObjectTable } from "../object/table.ts";
import { deletionColumns } from "./recoverable.ts";
import type { Trait } from "./trait.ts";

/** A controller's observation of a record. */
export const StatusCondition = defineSchema(
    schema.object({
        /** Whether the condition is true, or has not been established. */
        status: schema.enum(["true", "false", "unknown"]),
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(1),
        /** The machine-readable explanation. */
        reason: schema.string().min(1),
        /** The human-readable explanation. */
        message: schema.string(),
        /** The last change of condition status, in UTC epoch milliseconds. */
        lastTransitionAt: Instant,
    }),
);
/** A controller's observation of a record. */
export type StatusCondition = schema.Infer<typeof StatusCondition>;

/** Status conditions keyed by their unique domain-specific names. */
export const ConditionMap = defineSchema(schema.record(schema.string().min(1), StatusCondition));

/** A controller's report on one record: the generation it evaluated, its conditions and observed fields. */
export const Observation = defineSchema(
    schema.object({
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(0),
        /** The conditions by name, their transition times kept while their status stays. */
        conditions: schema.record(
            schema.string().min(1),
            StatusCondition.omit({ observedGeneration: true, lastTransitionAt: true }),
        ),
        /** The observed fields the controller writes. */
        fields: schema.record(schema.string(), schema.json()).exactOptional(),
    }),
);
/** A controller's report on one record. */
export type Observation = schema.Infer<typeof Observation>;

/** The method declarations of controlled objects. */
const controlledMethod: MethodBuilder<ControlledTable> = method;

/** The method declarations of controlled objects whose risky plans wait for approval. */
const approvalMethod: MethodBuilder<ApprovalTable> = method;

/** Record a controller's observation of its target at its generation. */
const observe = controlledMethod
    .mutation({ permission: null, isSystem: true, input: Observation })
    .handle(async (call) => {
        // merge the conditions and keep transition times while their status stays
        const { observedGeneration, conditions, fields } = call.input;
        const current = call.target.conditions;
        const merged = Object.fromEntries(
            Object.entries(conditions).map(([name, condition]) => [
                name,
                controlled.observe(current[name], { ...condition, observedGeneration }, call.now),
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
const finalize = controlledMethod
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
const approvePlan = approvalMethod
    .mutation({
        permission: null,
        isSystem: true,
        input: schema.object({
            /** The digest of the approved plan. */
            plan: Digest,
        }),
    })
    .handle((call) => call.update({ approvedPlan: call.input.plan }));

/** The table of controlled objects: the record columns, the controller's status and the deletion request. */
export type ControlledTable = ObjectTable<string, unknown, {}, { readonly controlled: true }>;

/** The table of controlled objects whose risky plans wait for approval. */
export type ApprovalTable = ObjectTable<
    string,
    unknown,
    {},
    { readonly controlled: { readonly approval: true } }
>;

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

/** Declare the plan waiting for approval and the digest an approver accepted. */
export function approvalColumns() {
    return {
        /** The plan waiting for approval, absent once applied. */
        plan: json("plan", Plan),
        /** The digest of the plan an approver accepted, applied once the controller plans it again. */
        approvedPlan: text("approved_plan"),
    };
}

/** Records a controller reconciles. */
export const controlled: Trait<ControlledDefinition> & {
    /** Record a controller's observation and keep the last transition time while the status stays. */
    observe(
        previous: StatusCondition | undefined,
        observation: Omit<StatusCondition, "lastTransitionAt">,
        now: number,
    ): StatusCondition;
} = {
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
    observe: (previous, observation, now) => ({
        ...observation,
        lastTransitionAt: previous?.status === observation.status ? previous.lastTransitionAt : now,
    }),
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
