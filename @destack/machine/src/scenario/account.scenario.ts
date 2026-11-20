import { account } from "@destack/account/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Join a member on their first device. */
export const joinMember = defineScenario({
    of: account,
    interaction: machineInteraction,
    name: "join-member",
    description: "join a member on their first device",
    given: { examples: [] },
    parameters: {},
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        {
            operation: "person.join",
            times: 1,
            step: { action: "join", members: 1, devices: 1 },
        },
    ],
});

/** Sync a team's fifty issues onto a member's new device. */
export const syncDevice = defineScenario({
    of: account,
    interaction: machineInteraction,
    name: "sync-device",
    description: "sync a team's fifty issues onto a member's new device",
    given: { examples: [] },
    parameters: { members: 3, issues: 50 },
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
            operation: "device.sync",
            times: 1,
            step: { action: "connect", member: "member-0", devices: 1 },
            then: [
                { spans: { name: "destack.replica.apply", host: "member-0.device-1" }, atMost: 3 },
            ],
        },
    ],
    then: [{ observe: { count: "issue", on: "member-0.device-1" }, is: "{issues}" }],
});
