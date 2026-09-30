import { InstallationBuild, InstallationSelection } from "../declare/installation.ts";
import { schema } from "@destack/schema";
import { SpaceDefinition } from "../declare/space.ts";

/** A stack's evaluation: the export, its arguments, and the space definition they evaluated to. */
export const StackEvaluation = schema.object({
    /** The exported definition or parameterised function. */
    export: schema.string().min(1),
    /** Arguments for a parameterised definition. */
    parameters: schema.record(schema.string(), schema.json()),
    /** The evaluated space definition. */
    definition: SpaceDefinition,
});
/** A stack's evaluation. */
export type StackEvaluation = schema.Infer<typeof StackEvaluation>;

/** A build submitted to the installation following it; a stack's carries its evaluation. */
export const Submission = schema.object({
    /** The release, repository reference or checkout the installation follows. */
    selection: InstallationSelection,
    /** The exact build evaluated. */
    build: InstallationBuild,
    /** The stack's evaluation, absent for an application. */
    evaluation: StackEvaluation.optional(),
});
/** A build submitted to the installation following it. */
export type Submission = schema.Infer<typeof Submission>;

/** Why a stack applied its revision, waits, or failed to apply it. */
export const StackReason = schema.enum([
    "Applied",
    "WaitingForResources",
    "AwaitingApproval",
    "Blocked",
    "ApplyFailed",
]);
/** Why a stack applied its revision, waits, or failed to apply it. */
export type StackReason = schema.Infer<typeof StackReason>;
