import { defineScenario } from "@destack/package/declare";
import { Machine } from "../machine/machine.ts";
import { machineInteraction } from "./interaction.ts";

/** Idle the owner's home without spans and wake it, while three enrolled computers follow it. */
export const idleAndWake = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "idle-and-wake",
    description: "idle a settled home without spans and wake it",
    given: { examples: [] },
    parameters: { minute: 60_000, halfHour: 1_800_000 },
    when: [
        {
            operation: "computer.enroll",
            times: 1,
            step: { action: "open", computer: "laptop" },
        },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
        { action: "run", milliseconds: "{minute}" },
        {
            operation: "machine.idle",
            times: 3,
            step: { action: "run", milliseconds: "{minute}" },
            then: [{ spans: { host: "home" }, count: 0 }],
        },
        {
            operation: "machine.wake",
            times: 3,
            step: { action: "jump", milliseconds: "{halfHour}" },
        },
    ],
});
