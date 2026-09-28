import { schema } from "@destack/schema";

/** A long-running operation. */
export type Operation<Result, Progress> = {
    /** The operation identifier. */
    id: string;
    /** The creation time, in UTC milliseconds. */
    createdAt: number;
    /** The last change time, in UTC milliseconds. */
    updatedAt: number;
    /** Whether cancellation was requested. */
    cancellationRequested: boolean;
    /** The latest progress. */
    progress: Progress;
} & (
    | { state: "running" }
    | { state: "succeeded"; completedAt: number; result: Result }
    | { state: "failed"; completedAt: number; error: OperationError }
    | { state: "cancelled"; completedAt: number }
);

/** A public operation failure. */
export const OperationError = schema.object({
    /** The failure code. */
    code: schema.string().min(1),
    /** The message. */
    message: schema.string(),
});

/** A public operation failure. */
export type OperationError = schema.Infer<typeof OperationError>;

/** The controls of an operation runner. */
export interface OperationContext<Progress> {
    /** The cancellation and deadline signal. */
    signal: AbortSignal;
    /** Report progress. */
    report(progress: Progress): void;
}

/** The schemas of an operation. */
export interface OperationDefinition<Result, Progress> {
    /** The completed result. */
    result: schema.Schema<Result>;
    /** The latest progress. */
    progress: schema.Schema<Progress>;
    /** The operation state. */
    operation: schema.Schema<Operation<Result, Progress>>;
}

/** Define an operation. */
export function defineOperation<Result, Progress>(
    result: schema.Schema<Result>,
    progress: schema.Schema<Progress>,
): OperationDefinition<Result, Progress> {
    // describe the common fields
    const common = schema.object({
        /** The operation identifier. */
        id: schema.uuid(),
        /** The creation time, in UTC milliseconds. */
        createdAt: schema.number().int(),
        /** The last change time, in UTC milliseconds. */
        updatedAt: schema.number().int(),
        /** Whether cancellation was requested. */
        cancellationRequested: schema.boolean(),
        /** The latest progress. */
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
