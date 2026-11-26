import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { Machine } from "../machine/machine.ts";
import { machineInteraction } from "./interaction.ts";

/** Enroll a computer in the owner's account. */
export const enrollComputer = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "enrollComputer",
    description: t`Enroll a computer in the owner's account.`,
    given: { examples: [] },
    parameters: {},
    when: [
        {
            operation: "computer.enroll",
            times: 1,
            step: { action: "open", computer: "laptop" },
        },
    ],
});

/** Idle a settled home followed by three computers. */
export const idleMachine = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "idle",
    description: t`Idle a settled home followed by three computers.`,
    given: { examples: [] },
    parameters: { minute: 60_000 },
    when: [
        { action: "open", computer: "laptop" },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
        { action: "run", milliseconds: "{minute}" },
        {
            operation: "machine.idle",
            times: 3,
            step: { action: "run", milliseconds: "{minute}" },
            then: [{ spans: { host: "home" }, count: 0 }],
        },
    ],
});

/** Wake an idle home after half an hour. */
export const wakeIdleMachine = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "wakeIdle",
    description: t`Wake an idle home after half an hour.`,
    given: { examples: [] },
    parameters: { minute: 60_000, halfHour: 1_800_000 },
    when: [
        { action: "open", computer: "laptop" },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
        { action: "run", milliseconds: "{minute}" },
        {
            operation: "machine.wake",
            times: 3,
            step: { action: "jump", milliseconds: "{halfHour}" },
        },
    ],
});
