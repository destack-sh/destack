import { defineScenario } from "@destack/package/declare";
import { terminalInteraction } from "../scenario/interaction.ts";
import { CommandLine } from "./line.ts";

/** The runs and warm-up runs. */
const PARAMETERS = { runs: 3, warmup: 1 };

/** Ask who is signed in. */
export const showWhoami = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "show-whoami",
    description: "ask who is signed in",
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "cli.whoami",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["whoami", "--json"] },
            then: [{ observe: { read: "output", field: "status" }, is: "authenticated" }],
        },
    ],
});

/** List the owner's spaces. */
export const listSpaces = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "list-spaces",
    description: "list the owner's spaces",
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "space.list",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["spaces", "--json"] },
            then: [{ observe: { read: "output", field: "name" }, is: ["home"] }],
        },
    ],
});

/** List the home's installations. */
export const listInstallations = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "list-installations",
    description: "list the home's installations",
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "installation.list",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["--space", "home", "installations", "--json"] },
            then: [
                {
                    observe: { read: "output", field: "name" },
                    is: [
                        "connection",
                        "inbox",
                        "message",
                        "observability",
                        "social",
                        "stack",
                        "start",
                        "vault",
                    ],
                },
            ],
        },
    ],
});
