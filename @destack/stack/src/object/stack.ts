import { BuildReference, InstallationSelection } from "../declare/installation.ts";
import { Digest, schema } from "@destack/schema";
import { StackDefinition } from "../declare/stack.ts";

/** A stack's evaluation: the export, its arguments, and the stack definition they evaluated to. */
export const StackEvaluation = schema.object({
    /** The exported definition or parameterised function. */
    export: schema.string().min(1),
    /** Arguments for a parameterised definition. */
    parameters: schema.jsonObject(),
    /** The evaluated stack definition. */
    definition: StackDefinition,
});
/** A stack's evaluation. */
export type StackEvaluation = schema.Infer<typeof StackEvaluation>;

/** A build submitted to the installation following it, with a stack's evaluation. */
export const Submission = schema.object({
    /** The release, repository reference or checkout the installation follows. */
    selection: InstallationSelection,
    /** The build to evaluate: a release, or a succeeded build of the space. */
    build: BuildReference,
    /** The stack's evaluation, absent for an application. */
    evaluation: StackEvaluation.exactOptional(),
    /** The digest of the space's stack the submission follows, absent for a person's own. */
    spaceStack: Digest.exactOptional(),
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
