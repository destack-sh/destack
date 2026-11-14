import { invitation } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Invite a person to own a new space with the owner, and accept as them. */
export const acceptIntoSpace = defineScenario({
    of: invitation,
    interaction: machineInteraction,
    name: "accept-into-space",
    description: "invite a person to own a new space with the owner, and accept as them",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "create", space: "team" },
        { action: "join", members: 1, devices: 0 },
        {
            operation: "invitation.create",
            times: 1,
            step: { action: "invite", member: "member-0", space: "team" },
        },
        {
            operation: "invitation.accept",
            times: 1,
            step: { action: "accept", member: "member-0" },
        },
    ],
});
