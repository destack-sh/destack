import { space } from "@destack/account/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Create a space in the owner's account until the home computer serves it. */
export const createUntilServed = defineScenario({
    of: space,
    interaction: machineInteraction,
    name: "create-until-served",
    description: "create a space in the owner's account until the home computer serves it",
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

/** Move a space from the owner's home to their laptop until the laptop serves it. */
export const moveToLaptop = defineScenario({
    of: space,
    interaction: machineInteraction,
    name: "move-to-laptop",
    description: "move a space from the owner's home to their laptop until the laptop serves it",
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
