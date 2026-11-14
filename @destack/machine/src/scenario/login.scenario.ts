import { login } from "@destack/account/native";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Sign three people up on the home computer, then sign the owner in again three times. */
export const signUpThenSignInAgain = defineScenario({
    of: login,
    interaction: machineInteraction,
    name: "sign-up-then-sign-in-again",
    description:
        "sign three people up on the home computer, then sign the owner in again three times",
    given: { examples: [] },
    parameters: { signIns: 3 },
    when: [
        {
            operation: "person.signUp",
            times: "{signIns}",
            step: { action: "sign-in", email: "person-{n}@scenario.test" },
        },
        {
            operation: "person.signIn",
            times: "{signIns}",
            step: { action: "sign-in", email: "owner@example.test" },
        },
    ],
});
