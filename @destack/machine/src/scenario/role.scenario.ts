import { role } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Create roles in a settled home followed by three computers. */
export const createRole = defineScenario({
    of: role,
    interaction: machineInteraction,
    name: "create-role",
    description: "create roles in a settled home followed by three computers",
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
    // expect the home's six roles plus the created ones
    then: [{ observe: { count: "role" }, is: "{6 + roles}" }],
});
