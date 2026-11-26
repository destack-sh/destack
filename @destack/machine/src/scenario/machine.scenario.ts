import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { Machine } from "../machine/machine.ts";
import { machineInteraction } from "./interaction.ts";

/** Wake a home holding a team's issues after a night. */
export const wakeMachineAfterNight = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "wakeAfterNight",
    description: t`Wake a home holding a team's issues after a night.`,
    given: { examples: [] },
    parameters: { members: 3, issues: 50, night: 28_800_000 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: 1 },
        { call: "team.create", as: "owner", input: { name: "Platform" }, bind: "team" },
        {
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

/** Restart a home computer followed by three computers. */
export const restartMachine = defineScenario({
    of: Machine,
    interaction: machineInteraction,
    name: "restart",
    description: t`Restart a home computer followed by three computers.`,
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
            // expect three space starts per restart
            then: [
                { spans: { name: "destack.space.start", host: "home" }, count: "{restarts * 3}" },
            ],
        },
    ],
});
