import { defineScenario } from "@destack/package/declare";
import { Feed } from "@destack/sync";
import { machineInteraction } from "./interaction.ts";

/** Fan a team's writes out to every member's devices, from sharing it to revoking a member. */
export const fanOutTeamWrites = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "fan-out-team-writes",
    description:
        "create and share a team, write its issues and comments, and revoke a member, while every member's devices follow it",
    given: { examples: [] },
    parameters: { members: 5, devices: 2, writes: 10 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: "{devices}" },
        {
            operation: "team.create",
            times: 1,
            step: { call: "team.create", as: "owner", input: { name: "Platform" }, bind: "team" },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{members * devices}",
                },
                { observe: { count: "team", on: "member-0.device-0" }, is: 0 },
            ],
        },
        {
            operation: "team.grant",
            times: "{members}",
            step: {
                call: "team.grant",
                as: "owner",
                input: { id: "$team", relation: "member", subject: "@member-{n}" },
                bind: "grants",
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: 35,
                },
            ],
        },
        {
            operation: "issue.create",
            times: "{writes}",
            step: {
                call: "issue.create",
                as: "member-0",
                input: { parentId: "$team", title: "Issue {n}" },
                bind: "issue",
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{members * devices}",
                },
            ],
        },
        {
            operation: "comment.create",
            times: "{writes}",
            step: {
                call: "comment.create",
                as: "member-1",
                input: { parentId: "$issue", text: "Comment {n}" },
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{members * devices}",
                },
            ],
        },
        {
            operation: "issue.update",
            times: "{writes}",
            step: {
                call: "issue.update",
                as: "member-0",
                input: { id: "$issue.{n}", state: "done" },
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{members * devices}",
                },
            ],
        },
        {
            operation: "issue.list",
            times: "{writes}",
            step: { call: "issue.list", as: "member-1", input: {} },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    count: 0,
                },
            ],
        },
        {
            operation: "team.revoke",
            times: 1,
            step: {
                call: "team.revoke",
                as: "owner",
                input: { id: "$team", relationshipId: "$grants.0" },
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: 35,
                },
            ],
        },
    ],
    then: [
        { observe: { count: "issue" }, is: "{writes}" },
        { observe: { count: "comment" }, is: "{writes}" },
        { observe: { count: "issue", on: "member-1.device-1" }, is: "{writes}" },
        { observe: { count: "issue", on: "member-0.device-0" }, is: 0 },
    ],
});

/** Fan bursts of messages in a channel out to every member's devices. */
export const fanOutMessageBursts = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "fan-out-message-bursts",
    description:
        "send bursts of ten messages a second in a channel while every member's devices follow it",
    given: { examples: [] },
    parameters: { members: 5, devices: 2, burst: 10, bursts: 3 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: "{devices}" },
        { call: "channel.create", as: "owner", input: { name: "general" }, bind: "channel" },
        {
            operation: "channel.grant",
            times: "{members}",
            step: {
                call: "channel.grant",
                as: "owner",
                input: { id: "$channel", relation: "member", subject: "@member-{n}" },
            },
        },
        {
            operation: "message.burst",
            times: "{bursts}",
            step: {
                call: "message.create",
                as: "member-1",
                input: { parentId: "$channel", text: "Message {n}" },
                repeat: "{burst}",
                rate: "{burst}",
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{burst * members * devices}",
                },
            ],
        },
    ],
    then: [
        { observe: { count: "message" }, is: "{burst * bursts}" },
        {
            observe: { count: "message", on: "member-4.device-1" },
            is: "{burst * bursts}",
        },
    ],
});

/** Fan three members typing into one page out to every member's devices. */
export const fanOutSharedTyping = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "fan-out-shared-typing",
    description:
        "type into one page with three members at once while every member's devices follow it",
    given: { examples: [] },
    parameters: { members: 5, devices: 2, typists: 3, keys: 10 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: "{devices}" },
        { call: "workspace.create", as: "owner", input: { name: "Docs" }, bind: "workspace" },
        {
            operation: "workspace.grant",
            times: "{members}",
            step: {
                call: "workspace.grant",
                as: "owner",
                input: { id: "$workspace", relation: "member", subject: "@member-{n}" },
            },
        },
        {
            call: "page.create",
            as: "owner",
            input: { parentId: "$workspace", title: "Meeting notes" },
            bind: "page",
        },
        {
            operation: "page.type",
            times: "{keys}",
            step: {
                action: "type",
                object: "page",
                on: "$page",
                field: "body",
                text: "a",
                members: "{typists}",
            },
        },
    ],
    then: [
        { observe: { count: "page" }, is: 1 },
        { observe: { count: "page", on: "member-4.device-1" }, is: 1 },
    ],
});

/** Fan a shape's drag at 60 Hz and its drop out to every member's devices. */
export const fanOutShapeDrag = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "fan-out-shape-drag",
    description:
        "drag a shape for a second at 60 Hz, then drop it, while every member's devices follow the file",
    given: { examples: [] },
    parameters: { members: 5, devices: 2, moves: 60, drags: 3 },
    when: [
        { action: "publish", fixture: "apps" },
        { action: "install", fixture: "apps" },
        { action: "join", members: "{members}", devices: "{devices}" },
        { call: "file.create", as: "owner", input: { name: "Design" }, bind: "file" },
        {
            operation: "file.grant",
            times: "{members}",
            step: {
                call: "file.grant",
                as: "owner",
                input: { id: "$file", relation: "member", subject: "@member-{n}" },
            },
        },
        { call: "shape.create", as: "owner", input: { parentId: "$file" }, bind: "shape" },
        {
            operation: "shape.drag",
            times: "{drags}",
            step: {
                action: "broadcast",
                object: "shape",
                on: "$shape",
                as: "member-0",
                event: { x: "{n}", y: "{n}" },
                repeat: "{moves}",
                rate: "{moves}",
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    count: 0,
                },
            ],
        },
        {
            operation: "shape.drop",
            times: "{drags}",
            step: {
                call: "shape.update",
                as: "member-0",
                input: { id: "$shape", x: "{n}", y: "{n}" },
            },
            then: [
                {
                    spans: { name: "destack.replica.apply", host: "member-*.device-*" },
                    atMost: "{members * devices}",
                },
            ],
        },
    ],
    then: [
        { observe: { count: "shape" }, is: 1 },
        { observe: { count: "shape", on: "member-4.device-1" }, is: 1 },
    ],
});
