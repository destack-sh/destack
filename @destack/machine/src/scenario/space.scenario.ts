import { space } from "@destack/account/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Create a space until the home computer serves it. */
export const createSpace = defineScenario({
    of: space,
    interaction: machineInteraction,
    name: "create-space",
    description: "create a space until the home computer serves it",
    given: { examples: [] },
    parameters: {},
    when: [
        {
            operation: "space.create",
            times: 1,
            step: { action: "create", space: "team" },
            then: [{ spans: { name: "destack.space.start", host: "home" }, count: 1 }],
        },
    ],
});

/** Move a space to the owner's laptop until it serves it. */
export const moveSpace = defineScenario({
    of: space,
    interaction: machineInteraction,
    name: "move-space",
    description: "move a space to the owner's laptop until it serves it",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "create", space: "team" },
        { action: "open", computer: "laptop" },
        {
            operation: "space.move",
            times: 1,
            step: { action: "move", space: "team", computer: "laptop" },
            then: [{ spans: { name: "destack.space.start", host: "laptop" }, count: 1 }],
        },
    ],
});
