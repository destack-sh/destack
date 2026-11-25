import { canonicalize, Instant, schema } from "@destack/schema";
import type { Call, ObjectReference } from "@destack/sync";
import type { ObjectType } from "./object.ts";

/** The condition of a controlled object's last reconcile succeeding, as Crossplane's Synced. */
export const SYNCED = "Synced";

/** The Synced decision of a controlled object whose last reconcile succeeded. */
export const SYNCED_DECISION = {
    status: "true",
    reason: "ReconcileSuccess",
    message: "",
} as const;

/** The ready reason of a controlled object whose plan waits for approval. */
export const AWAITING_APPROVAL = "AwaitingApproval";

/** How far a controlled object is with its latest change, as kstatus reads its conditions. */
export type Progress = "current" | "waiting" | "approval" | "failed";

/** A controller's decision on a condition of a record: whether it holds, and why. */
export const Decision = schema.object({
    /** Whether the condition is true, or has not been established. */
    status: schema.enum(["true", "false", "unknown"]),
    /** The machine-readable explanation. */
    reason: schema.string().min(1),
    /** The human-readable explanation. */
    message: schema.string(),
});
/** A controller's decision on a condition of a record. */
export type Decision = schema.Infer<typeof Decision>;

/** A condition of a record as its controller observed it, as Kubernetes' metav1.Condition. */
export const Condition = Object.assign(
    Decision.extend({
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(1),
        /** The last change of condition status, in UTC epoch milliseconds. */
        lastTransitionAt: Instant,
    }),
    {
        /** Record a decision at a generation, its last transition time unchanged while stable. */
        observe(
            previous: Condition | undefined,
            observed: Omit<Condition, "lastTransitionAt">,
            now: number,
        ): Condition {
            return {
                ...observed,
                lastTransitionAt:
                    previous?.status === observed.status ? previous.lastTransitionAt : now,
            };
        },
    },
);
/** A condition of a record as its controller observed it. */
export type Condition = schema.Infer<typeof Condition>;

/** A controller's report on one record: its decisions by condition name and its observed fields. */
const observation = schema.object({
    /** The decisions by condition name. */
    conditions: schema.record(schema.string().min(1), Decision),
    /** The observed fields the controller writes. */
    fields: schema.jsonObject().exactOptional(),
});

/** A controller's report on one record, and how a store records it. */
export const Observation = Object.assign(observation, {
    /** Describe the call recording an observation of a row at its current generation. */
    call(object: ObjectType, row: ControlledRecord, report: Observation): Call {
        return object.call("observe", {
            ...object.target({ scope: row.scope, id: row.id }),
            observedGeneration: row.generation,
            conditions: report.conditions,
            ...(report.fields === undefined ? {} : { fields: report.fields }),
        });
    },

    /** Report whether a record already has an observation. */
    isRecorded(record: Status & Readonly<Record<string, unknown>>, report: Observation): boolean {
        return (
            Object.entries(report.conditions).every(([name, decision]) => {
                const current = record.conditions[name];

                return (
                    current?.observedGeneration === record.generation &&
                    current.status === decision.status &&
                    current.reason === decision.reason &&
                    current.message === decision.message
                );
            }) &&
            Object.entries(report.fields ?? {}).every(
                ([name, value]) =>
                    canonicalize(record[name] ?? null) === canonicalize(value ?? null),
            )
        );
    },
});
/** A controller's report on one record, its fields typed as the controller writes them. */
export type Observation = Omit<schema.Infer<typeof observation>, "fields"> & {
    /** The observed fields the controller writes. */
    readonly fields?: Readonly<Record<string, unknown>>;
};

/** The status of a controlled record: its desired generation and its conditions by name. */
export interface Status {
    /** The desired generation. */
    readonly generation: number;
    /** The conditions by name. */
    readonly conditions: Readonly<Record<string, Condition>>;
}

/** A record of a controlled type: its identity, its status and its other fields. */
export type ControlledRecord = Status &
    Pick<ObjectReference, "scope" | "id"> &
    Readonly<Record<string, unknown>>;

/** The conditions of controlled records, and how far each is with its latest change. */
export const Status = {
    /** The conditions of a record by name, as its conditions column stores them. */
    conditions: schema.record(schema.string().min(1), Condition),

    /** Report whether a record's named condition was observed at its current generation. */
    isCurrent(status: Status, name: string): boolean {
        return status.conditions[name]?.observedGeneration === status.generation;
    },

    /** Read how far a record is with its latest change, from its Synced and Ready. */
    progress(status: Status): Progress {
        const ready = status.conditions["Ready"];
        const synced = status.conditions[SYNCED];

        // wait until a pass for the latest change ended
        if (synced === undefined || !Status.isCurrent(status, SYNCED)) {
            return "waiting";
        }
        // fail on a lasting failure of that pass
        else if (synced.status === "false") {
            return "failed";
        }
        // wait on a person approving its plan
        else if (ready?.reason === AWAITING_APPROVAL && Status.isCurrent(status, "Ready")) {
            return "approval";
        }
        // end once ready at the latest change
        else if (ready?.status === "true" && Status.isCurrent(status, "Ready")) {
            return "current";
        }
        // wait on what its ready condition names
        else {
            return "waiting";
        }
    },

    /** Read what a record reports with its progress: its failure, else what Ready says. */
    message(status: Status): string {
        return Status.condition(status)?.message ?? "";
    },

    /** Read the condition a record reports by: its failure, else its ready one. */
    condition(status: Status): Condition | undefined {
        const { Ready: ready, [SYNCED]: synced } = status.conditions;

        return Status.progress(status) === "failed" ? synced : ready;
    },
};

/** The conditions a controller reports on its records under declared names, read typed by name. */
export class ConditionSet<const Name extends string> {
    /** The names the controller reports. */
    readonly names: readonly Name[];

    /** Declare the names a controller reports. */
    constructor(names: readonly Name[]) {
        this.names = names;
    }

    /** Read a record's condition by name, absent until its controller reports it. */
    read(record: Pick<Status, "conditions">, name: Name): Condition | undefined {
        return record.conditions[name];
    }
}
