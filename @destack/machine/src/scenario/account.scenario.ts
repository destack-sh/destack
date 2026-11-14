import { account } from "@destack/account/object";
import { defineScenario } from "@destack/package/declare";
import { machineInteraction } from "./interaction.ts";

/** Sign a person in with a first device following the owner's home and its app. */
export const joinOnFirstDevice = defineScenario({
    of: account,
    interaction: machineInteraction,
    name: "join-on-first-device",
    description: "sign a person in with a first device following the owner's home and its app",
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

/** Open a member's new device on a team's fifty issues, syncing them first. */
export const syncNewDevice = defineScenario({
    of: account,
    interaction: machineInteraction,
    name: "sync-new-device",
    description: "open a member's new device on a team's fifty issues, syncing them first",
    given: { examples: [] },
    parameters: { members: 3, issues: 50 },
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
