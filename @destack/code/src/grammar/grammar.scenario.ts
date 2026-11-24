import { defineScenario } from "@destack/package/declare";
import { codeInteraction } from "../scenario/code.ts";
import { LezerGrammar } from "./lezer.ts";

/** Open, type in and scroll through a file of about a megabyte. */
export const editLargeFile = defineScenario({
    of: LezerGrammar,
    interaction: codeInteraction,
    name: "edit-large-file",
    description:
        "parse a view of 27,000 lines, its first screen and the whole of it, then type in it and read the tokens of screens",
    given: { examples: [] },
    parameters: { times: 200 },
    when: [
        { step: { action: "write", lines: 27_000 } },
        { operation: "code.screen", times: "{times}", step: { action: "parse", extent: "screen" } },
        { operation: "code.parse", times: 5, step: { action: "parse", extent: "whole" } },
        { operation: "code.type", times: "{times}", step: { action: "type" } },
        { operation: "code.tokens", times: "{times}", step: { action: "tokens" } },
    ],
    then: [{ observe: { count: "lines" }, is: 27_000 }],
});
