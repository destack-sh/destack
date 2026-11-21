import { defineScenario } from "@destack/package/declare";
import { repository } from "@destack/registry/object";
import { forgeInteraction } from "./interaction.ts";

/** The runs and warm-up runs of each operation. */
const PARAMETERS = { runs: 3, warmup: 1 };

/** The local commits: one per push, and one more for the clones to pull. */
const COMMITS = PARAMETERS.runs + PARAMETERS.warmup + 1;

/** Work on a platform repository with git. */
export const workOnRepository = defineScenario({
    of: repository,
    interaction: forgeInteraction,
    name: "work-on-repository",
    description: "create a platform repository, push to it, clone it and pull it",
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "repository.create",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "create" },
        },
        { action: "commit", count: COMMITS },
        {
            operation: "repository.push",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "push" },
        },
        {
            operation: "repository.clone",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "clone" },
            then: [{ observe: { read: "synced" }, is: true }],
        },
        { action: "push" },
        {
            operation: "repository.pull",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "pull" },
            then: [{ observe: { read: "synced" }, is: true }],
        },
    ],
});
