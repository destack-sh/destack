import { canonicalize, Instant, schema } from "@destack/schema";
import type { Call } from "@destack/sync";
import type { ObjectType } from "./object.ts";

/** A controller's observation of a record. */
export const StatusCondition = Object.assign(
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
    {
        /** Record an observed condition, its last transition time unchanged while stable. */
        observe(
            previous: StatusCondition | undefined,
            observation: Omit<StatusCondition, "lastTransitionAt">,
            now: number,
        ): StatusCondition {
            return {
                ...observation,
                lastTransitionAt:
                    previous?.status === observation.status ? previous.lastTransitionAt : now,
            };
        },
    },
);
/** A controller's observation of a record. */
export type StatusCondition = schema.Infer<typeof StatusCondition>;

/** A controller's condition: whether it applies and why, before generation and time stamps. */
export const Condition = StatusCondition.omit({ observedGeneration: true, lastTransitionAt: true });
/** A condition a controller decides. */
export type Condition = schema.Infer<typeof Condition>;

/** Status conditions keyed by their unique domain-specific names. */
export const ConditionMap = schema.record(schema.string().min(1), StatusCondition);

/** A record controllers observe: its desired generation and their named conditions. */
export interface ObservedRecord {
    /** The scope the record lives in. */
    readonly scope: string;
    /** The record's identifier. */
    readonly id: string;
    /** The desired generation. */
    readonly generation: number;
    /** The conditions by name. */
    readonly conditions: Readonly<Record<string, StatusCondition>>;
}

/** A controller's report on one record: the evaluated generation, conditions and fields. */
export const Observation = Object.assign(
    schema.object({
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(0),
        /** The conditions by name, their transition times fixed while their status stays. */
        conditions: schema.record(schema.string().min(1), Condition),
        /** The observed fields the controller writes. */
        fields: schema.jsonObject().exactOptional(),
    }),
    {
        /** Describe the call recording an observation of a row at its current generation. */
        call(
            object: ObjectType,
            row: ObservedRecord & Readonly<Record<string, unknown>>,
            observation: ObservationReport,
        ): Call {
            return object.call("observe", {
                ...object.target({ scope: row.scope, id: row.id }),
                observedGeneration: row.generation,
                conditions: observation.conditions,
                ...(observation.fields === undefined ? {} : { fields: observation.fields }),
            });
        },

        /** Report whether a record's named condition was observed at its current generation. */
        isCurrent(record: Omit<ObservedRecord, "scope" | "id">, name: string): boolean {
            return record.conditions[name]?.observedGeneration === record.generation;
        },

        /** Report whether a record already has an observation. */
        isRecorded(
            record: ObservedRecord & Readonly<Record<string, unknown>>,
            observation: ObservationReport,
        ): boolean {
            return (
                Object.entries(observation.conditions).every(([name, condition]) => {
                    const current = record.conditions[name];

                    return (
                        current?.observedGeneration === record.generation &&
                        current.status === condition.status &&
                        current.reason === condition.reason &&
                        current.message === condition.message
                    );
                }) &&
                Object.entries(observation.fields ?? {}).every(
                    ([name, value]) =>
                        canonicalize(record[name] ?? null) === canonicalize(value ?? null),
                )
            );
        },
    },
);

/** A controller's report on one record. */
export type Observation = schema.Infer<typeof Observation>;

/** A controller's unrecorded report on one record: its conditions and observed fields. */
export interface ObservationReport {
    /** The conditions by name. */
    readonly conditions: Observation["conditions"];
    /** The observed fields the controller writes. */
    readonly fields?: Readonly<Record<string, unknown>>;
}

/** The conditions a controller reports on its records under declared names, read typed by name. */
export class ConditionSet<const Name extends string> {
    /** The names the controller reports. */
    readonly names: readonly Name[];

    /** Declare the names a controller reports. */
    constructor(names: readonly Name[]) {
        this.names = names;
    }

    /** Read a record's condition by name, absent until its controller reports it. */
    read(
        record: { readonly conditions: Readonly<Record<string, StatusCondition>> },
        name: Name,
    ): StatusCondition | undefined {
        return record.conditions[name];
    }
}
