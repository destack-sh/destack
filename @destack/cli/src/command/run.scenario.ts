import { defineScenario } from "@destack/package/declare";
import { terminalInteraction } from "../scenario/interaction.ts";
import { CommandLine } from "./run.ts";

/** Run the common command lines against the owner's home, each warm after its first run. */
export const runCommonCommands = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "run-common-commands",
    description: "ask who is signed in, list the owner's spaces and the home's installations",
    given: { examples: [] },
    parameters: { runs: 3, warmup: 1 },
    when: [
        {
            operation: "cli.whoami",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["whoami", "--json"] },
            then: [{ observe: { read: "output", field: "status" }, is: "authenticated" }],
        },
        {
            operation: "space.list",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["spaces", "--json"] },
            then: [{ observe: { read: "output", field: "name" }, is: ["home"] }],
        },
        {
            operation: "installation.list",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["--space", "home", "installations", "--json"] },
            then: [
                {
                    observe: { read: "output", field: "name" },
                    is: [
                        "inbox",
                        "message",
                        "social",
                        "vault",
                        "connection",
                        "observability",
                        "stack",
                        "start",
                    ],
                },
            ],
        },
    ],
});
