import { schema } from "@destack/schema";

/** A current operation value, including its terminal outcome when complete. */
export type Operation<Result, Progress> = {
    /** Operation identifier. */
    id: string;
    /** Creation time in UTC milliseconds. */
    createdAt: number;
    /** Last change time in UTC milliseconds. */
    updatedAt: number;
    /** Whether the runner received a cancellation request. */
    cancellationRequested: boolean;
    /** Latest progress. */
    progress: Progress;
} & (
    | { state: "running" }
    | { state: "succeeded"; completedAt: number; result: Result }
    | { state: "failed"; completedAt: number; error: OperationError }
    | { state: "cancelled"; completedAt: number }
);

/** A public operation failure, excluding internal exception details. */
export const OperationError = schema.object({
    /** Stable failure code. */
    code: schema.string().min(1),
    /** Public explanation. */
    message: schema.string(),
});

/** A public operation failure. */
export type OperationError = schema.Infer<typeof OperationError>;

/** Controls supplied to one operation runner. */
export interface OperationContext<Progress> {
    /** Cancellation and deadline notification. */
    signal: AbortSignal;
    /** Publish the latest validated progress. */
    report(progress: Progress): void;
}

/** Schemas shared by an operation's procedures and runner. */
export interface OperationDefinition<Result, Progress> {
    /** The completed result. */
    result: schema.Schema<Result>;
    /** The latest progress. */
    progress: schema.Schema<Progress>;
    /** The observable operation state. */
    operation: schema.Schema<Operation<Result, Progress>>;
}

/** Define a long-running operation with typed progress and result values. */
export function defineOperation<Result, Progress>(
    result: schema.Schema<Result>,
    progress: schema.Schema<Progress>,
): OperationDefinition<Result, Progress> {
    // describe fields shared by running and completed operations
    const common = schema.object({
        /** The operation identifier. */
        id: schema.uuid(),
        /** Creation time in UTC milliseconds. */
        createdAt: schema.number().int(),
        /** Last change time in UTC milliseconds. */
        updatedAt: schema.number().int(),
        /** Whether cancellation has been requested. */
        cancellationRequested: schema.boolean(),
        /** Latest domain-specific progress. */
        progress: progress.nonoptional(),
    });

    const operation = schema.discriminatedUnion("state", [
        common.extend({ state: schema.literal("running") }),
        common.extend({
            state: schema.literal("succeeded"),
            completedAt: schema.number().int(),
            result: result.nonoptional(),
        }),
        common.extend({
            state: schema.literal("failed"),
            completedAt: schema.number().int(),
            error: OperationError,
        }),
        common.extend({
            state: schema.literal("cancelled"),
            completedAt: schema.number().int(),
        }),
    ]) as schema.Schema<Operation<Result, Progress>>;

    return { result, progress, operation };
}
