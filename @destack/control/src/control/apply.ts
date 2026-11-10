import { Digest, schema } from "@destack/schema";
import { defineWorkflow, NO_OUTPUT } from "@destack/workflow";

/** The input of a run applying the universe's declaration: the declaration's digest. */
export const ApplyInput = schema.object({
    /** The digest of the declaration the run applies, the key of its one live run. */
    digest: Digest,
});

/** Apply the universe's declared state step by step: residencies, machines and the platform. */
export const apply = defineWorkflow({
    name: "apply",
    scope: "universe",
    input: ApplyInput,
    output: NO_OUTPUT,
});
