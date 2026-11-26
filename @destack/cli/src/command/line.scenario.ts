import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { terminalInteraction } from "../scenario/interaction.ts";
import { CommandLine } from "./line.ts";

/** The runs and warm-up runs. */
const PARAMETERS = { runs: 3, warmup: 1 };

/** Ask who is signed in. */
export const showWhoami = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "showWhoami",
    description: t`Ask who is signed in.`,
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
    name: "listSpaces",
    description: t`List the owner's spaces.`,
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
    name: "listInstallations",
    description: t`List the home's installations.`,
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
                        "forge",
                        "home",
                        "inbox",
                        "message",
                        "observability",
                        "social",
                        "stack",
                        "vault",
                    ],
                },
            ],
        },
    ],
});

/** The credential request git sends its helper for a repository at the universe's origin. */
const CREDENTIAL_REQUEST = "capability[]=authtype\nprotocol=https\nhost=universe.test\n\n";

/** Answer git's credential request. */
export const answerGitCredential = defineScenario({
    of: CommandLine,
    interaction: terminalInteraction,
    name: "answerGitCredential",
    description: t`Answer git's credential request with the owner's token.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        {
            operation: "git.credential",
            times: "{runs}",
            warmup: "{warmup}",
            step: { action: "run", words: ["git", "credential", "get"], input: CREDENTIAL_REQUEST },
        },
    ],
});
