import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, PackageId } from "@destack/package";
import { Call } from "@destack/sync";
import type { Schedule } from "../schedule/schedule.ts";
import type { Webhook } from "../webhook/webhook.ts";
import type { Watch } from "../watch/watch.ts";

/** The trigger kinds. */
export const TRIGGER_KINDS = ["schedule", "webhook", "watch"] as const;

/** A declared cause of runs. */
export type Trigger = Schedule | Webhook | Watch<any>;

/** The declared triggers of one kind. */
export type TriggerOf<Kind extends Trigger["kind"]> = Extract<Trigger, { readonly kind: Kind }>;

/** A call sent to run later, on the lent authority of the caller whose call sent it. */
const SENT = schema.object({
    /** A sent call. */
    cause: schema.literal("send"),
    /** The object method call the run makes. */
    call: Call,
    /** When the call runs, in UTC epoch milliseconds; when the cell records it when absent. */
    at: schema.number().int().nonnegative().optional(),
    /** The lending of the sending caller's authority, so the call runs on that caller's behalf. */
    delegation: schema.string().min(1).optional(),
});

/** A call a verified webhook delivery runs, once per route path and delivery. */
const DELIVERED = schema.object({
    /** A webhook's delivery. */
    cause: schema.literal("webhook"),
    /** The object method call the run makes. */
    call: Call,
    /** The package declaring the webhook. */
    packageId: PackageId,
    /** The webhook's name within its package. */
    trigger: DeclarationName,
    /** The route path the delivery arrived at and the sender's delivery identifier. */
    deliveryId: schema.string().min(1),
});

/** A call a watched change runs, once per log position and snapshot row. */
const WATCHED = schema.object({
    /** A watch's change. */
    cause: schema.literal("watch"),
    /** The object method call the run makes. */
    call: Call,
    /** The package declaring the watch. */
    packageId: PackageId,
    /** The watch's name within its package. */
    trigger: DeclarationName,
    /** The log epoch of the change. */
    epoch: schema.string().min(1),
    /** The log sequence of the change. */
    sequence: schema.number().int().nonnegative(),
    /** The row the watch's snapshot delivers at the change's position. */
    key: schema.string().min(1).optional(),
});

/** A call to run later, as a workload records it in its space's cell: one shape per cause, each the cause's own columns. */
export const RunRequest = defineSchema(
    schema.discriminatedUnion("cause", [SENT, DELIVERED, WATCHED]),
);
/** A call to run later, as a workload records it. */
export type RunRequest = schema.Infer<typeof RunRequest>;

/** How a request reaches the cell. */
export interface RunDelivery {
    /** The request recording the run once however often it is delivered, a new one when absent. */
    readonly requestId?: string;
    /** Abort the delivery. */
    readonly signal?: AbortSignal;
}

/** The cell recording an installation's runs, as its workload reaches it. */
export interface RunClient {
    /** Record a call to run later, once per request and per webhook delivery or watched change. */
    send(request: RunRequest, delivery?: RunDelivery): Promise<void>;
}

/** Why a run failed or was skipped. */
export const RunError = defineSchema(
    schema.object({
        /** The stable failure code, such as the error's code. */
        code: schema.string().min(1),
        /** The failure message. */
        message: schema.string(),
    }),
);
/** Why a run failed or was skipped. */
export type RunError = schema.Infer<typeof RunError>;

/** Describe a failure by its code, or its error name, and its message. */
export function runError(error: unknown): RunError {
    // name errors by their code when they carry one
    if (error instanceof Error) {
        const code = "code" in error && typeof error.code === "string" ? error.code : error.name;

        return { code, message: error.message };
    }

    return { code: "THROWN", message: String(error) };
}
