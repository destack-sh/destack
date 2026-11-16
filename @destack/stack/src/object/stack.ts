import { schema } from "@destack/schema";
import { AWAITING_APPROVAL, PackageDirectory } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import { StackDefinition } from "../declare/index.ts";

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

/** A build for the installation following it to deploy, with a stack's export and arguments. */
export const Submission = schema.object({
    /** The tag, release, reference or checkout the installation follows, absent for the host's. */
    follows: ObjectReference.exactOptional(),
    /** The package directory within a followed repository or checkout. */
    directory: PackageDirectory.exactOptional(),
    /** The succeeded build of the space to deploy. */
    build: schema.identifier("build"),
    /** The export the space evaluates and its arguments, absent for an application. */
    stack: StackEvaluation.pick({ export: true, parameters: true }).exactOptional(),
});
/** A build submitted to the installation following it. */
export type Submission = schema.Infer<typeof Submission>;

/** Why a stack applied its revision or waits, its failures recorded as Synced. */
export const StackReadiness = schema.enum([
    "Applied",
    "Applying",
    "WaitingForResources",
    AWAITING_APPROVAL,
]);
/** Why a stack applied its revision or waits, its failures recorded as Synced. */
export type StackReadiness = schema.Infer<typeof StackReadiness>;
