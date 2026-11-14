import { role } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Create roles in the owner's settled home while three enrolled computers follow it. */
export const createWhileComputersFollow = defineScenario({
    of: role,
    interaction: machineInteraction,
    name: "create-while-computers-follow",
    description:
        "create roles in the owner's settled home while three enrolled computers follow it",
    given: { examples: [] },
    parameters: { roles: 3, settle: 60_000 },
    when: [
        { action: "open", computer: "laptop" },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
        { action: "run", milliseconds: "{settle}" },
        {
            operation: "role.create",
            times: "{roles}",
            step: {
                call: "role.create",
                as: "owner",
                input: {
                    name: "role-{n}",
                    description: "A role the scenario creates",
                    permissions: [],
                },
            },
        },
    ],
    // expect the six roles a home starts with beside the created ones
    then: [{ observe: { count: "role" }, is: "{6 + roles}" }],
});
