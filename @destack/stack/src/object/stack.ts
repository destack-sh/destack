import { schema } from "@destack/schema";
import { InstallationSelection } from "@destack/object";
import { BuildReference, StackDefinition } from "../declare/index.ts";

/** A stack's evaluation: its export, arguments, settings, and the definition they give. */
export const StackEvaluation = schema.object({
    /** The exported definition or parameterised function. */
    export: schema.string().min(1),
    /** Arguments for a parameterised definition. */
    parameters: schema.jsonObject(),
    /** The setting values of the space's stack the export read, none for another package's. */
    settings: schema.record(schema.string(), schema.json()),
    /** The evaluated stack definition. */
    definition: StackDefinition,
});
/** A stack's evaluation. */
export type StackEvaluation = schema.Infer<typeof StackEvaluation>;

/** A build submitted to the installation following it, with a stack's export and arguments. */
export const Submission = schema.object({
    /** The release, repository reference or checkout the installation follows. */
    selection: InstallationSelection,
    /** The build to evaluate: a release, or a succeeded build of the space. */
    build: BuildReference,
    /** The export the space evaluates and its arguments, absent for an application. */
    stack: StackEvaluation.pick({ export: true, parameters: true }).exactOptional(),
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
