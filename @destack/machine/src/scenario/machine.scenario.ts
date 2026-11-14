import { defineScenario } from "@destack/package/declare";
import { Machine } from "../machine/machine.ts";
import { machineInteraction } from "./interaction.ts";

/** Idle the owner's home and wake it, while three enrolled computers follow it. */
export const idleAndWake = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "idle-and-wake",
    description:
        "idle the owner's settled home for minutes and wake it after half hours, three computers following",
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

/** Wake the owner's home after a night, holding a team's issues. */
export const wakeAfterNight = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "wake-after-night",
    description: "wake the owner's home after a night, holding a team's fifty issues",
    given: { examples: [] },
    parameters: { members: 3, issues: 50, night: 28_800_000 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: 1 },
        { call: "team.create", as: "owner", input: { name: "Platform" }, bind: "team" },
        {
            operation: "team.grant",
            times: "{members}",
            step: {
                call: "team.grant",
                as: "owner",
                input: { id: "$team", relation: "member", subject: "@member-{n}" },
            },
        },
        {
            call: "issue.create",
            as: "member-0",
            input: { parentId: "$team", title: "Issue {n}" },
            repeat: "{issues}",
        },
        {
            operation: "machine.wake",
            times: 1,
            step: { action: "jump", milliseconds: "{night}" },
        },
    ],
    then: [{ observe: { count: "issue" }, is: "{issues}" }],
});

/** Restart the owner's home computer while three computers follow it, until it serves again. */
export const restartAndServeAgain = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "restart-and-serve-again",
    description:
        "restart the owner's home computer while three computers follow it, until it serves the home again",
    given: { examples: [] },
    parameters: { restarts: 2 },
    when: [
        { action: "open", computer: "laptop" },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
        {
            operation: "machine.restart",
            times: "{restarts}",
            step: { action: "restart" },
            // expect each restart to start the home's three spaces
            then: [
                { spans: { name: "destack.space.start", host: "home" }, count: "{restarts * 3}" },
            ],
        },
    ],
});
