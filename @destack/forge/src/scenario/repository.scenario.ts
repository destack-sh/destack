import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { repository } from "@destack/registry/object";
import { forgeInteraction } from "./interaction.ts";

/** The runs and warm-up runs of each operation. */
const PARAMETERS = { runs: 3, warmup: 1 };

/** Work on a platform repository with git. */
export const workOnRepository = defineScenario({
    of: repository,
    interaction: forgeInteraction,
    name: "workOn",
    description: t`Create a platform repository, push to it, clone it and pull it.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "repository.create",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "create" },
        },
        // commit once per push, and once more for the clones to pull
        { action: "commit", count: "{runs + warmup + 1}" },
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
