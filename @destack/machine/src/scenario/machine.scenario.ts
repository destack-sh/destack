import { defineScenario } from "@destack/package/declare";
import { Machine } from "../machine/machine.ts";
import { machineInteraction } from "./interaction.ts";

/** Wake the owner's home after a night, holding a team's issues. */
export const wakeAfterNight = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "wake-after-night",
    description: "wake a home after a night holding its issues",
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
    description: "restart a home computer and start its three spaces again",
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
