import { login } from "@destack/account/native";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Sign three new people up. */
export const signUp = defineScenario({
    of: login,
    interaction: machineInteraction,
    name: "sign-up",
    description: "sign three new people up",
    given: { examples: [] },
    parameters: { signUps: 3 },
    when: [
        {
            operation: "person.signUp",
            times: "{signUps}",
            step: { action: "sign-in", email: "person-{n}@scenario.test" },
        },
    ],
});

/** Sign the owner in three times. */
export const signIn = defineScenario({
    of: login,
    interaction: machineInteraction,
    name: "sign-in",
    description: "sign the owner in three times",
    given: { examples: [] },
    parameters: { signIns: 3 },
    when: [
        {
            operation: "person.signIn",
            times: "{signIns}",
            step: { action: "sign-in", email: "owner@example.test" },
        },
    ],
});
