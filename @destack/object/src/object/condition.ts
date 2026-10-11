import { canonicalize, defineSchema, Instant, schema } from "@destack/schema";

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

/** A condition a controller decides: whether it holds, and why, before its record stamps the generation and transition time. */
export const Condition = defineSchema(
    StatusCondition.omit({ observedGeneration: true, lastTransitionAt: true }),
);
/** A condition a controller decides. */
export type Condition = schema.Infer<typeof Condition>;

/** Status conditions keyed by their unique domain-specific names. */
export const ConditionMap = defineSchema(schema.record(schema.string().min(1), StatusCondition));

/** A record a controller observes: its desired and observed generations and its conditions by name. */
export interface ObservedRecord {
    /** The scope the record lives in. */
    readonly scope: string;
    /** The record's identifier. */
    readonly id: string;
    /** The desired generation. */
    readonly generation: number;
    /** The last generation a controller observed. */
    readonly observedGeneration: number;
    /** The conditions by name. */
    readonly conditions: Readonly<Record<string, StatusCondition>>;
}

/** A controller's report on one record: the generation it evaluated, its conditions and observed fields. */
export const Observation = Object.assign(
    defineSchema(
        schema.object({
            /** The desired generation the controller evaluated. */
            observedGeneration: schema.number().int().min(0),
            /** The conditions by name, their transition times kept while their status stays. */
            conditions: schema.record(schema.string().min(1), Condition),
            /** The observed fields the controller writes. */
            fields: schema.record(schema.string(), schema.json()).exactOptional(),
        }),
    ),
    {
        /** Report whether a record has an observation already: its current generation observed, and the conditions and fields as observed. */
        isRecorded(
            record: ObservedRecord & Readonly<Record<string, unknown>>,
            observation: ObservationReport,
        ): boolean {
            return (
                record.observedGeneration === record.generation &&
                Object.entries(observation.conditions).every(([name, condition]) => {
                    const current = record.conditions[name];

                    return (
                        current?.status === condition.status &&
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

/** A controller's report on one record before it is recorded: its conditions and the fields it observed. */
export interface ObservationReport {
    /** The conditions by name. */
    readonly conditions: Observation["conditions"];
    /** The observed fields the controller writes. */
    readonly fields?: Readonly<Record<string, unknown>>;
}

/** The conditions a controller reports on its records under declared names, read typed by name. */
export class Conditions<const Name extends string> {
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

/** Record a controller's observation of a condition, keeping its last transition time while the status stays. */
export function observeCondition(
    previous: StatusCondition | undefined,
    observation: Omit<StatusCondition, "lastTransitionAt">,
    now: number,
): StatusCondition {
    return {
        ...observation,
        lastTransitionAt: previous?.status === observation.status ? previous.lastTransitionAt : now,
    };
}
