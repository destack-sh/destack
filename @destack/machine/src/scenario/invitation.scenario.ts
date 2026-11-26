import { t } from "@destack/locale";
import { invitation } from "@destack/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Invite a person to own a new space. */
export const createInvitation = defineScenario({
    of: invitation,
    interaction: machineInteraction,
    name: "create",
    description: t`Invite a person to own a new space.`,
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
    ],
});

/** Accept an invitation to own a new space. */
export const acceptInvitation = defineScenario({
    of: invitation,
    interaction: machineInteraction,
    name: "accept",
    description: t`Accept an invitation to own a new space.`,
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "create", space: "team" },
        { action: "join", members: 1, devices: 0 },
        { action: "invite", member: "member-0", space: "team" },
        {
            operation: "invitation.accept",
            times: 1,
            step: { action: "accept", member: "member-0" },
        },
    ],
});
