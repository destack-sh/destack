import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { Feed } from "@destack/sync/dataflow";
import { machineInteraction } from "./interaction.ts";

/** The members, their devices and the writes. */
const PARAMETERS = { members: 2, devices: 2, writes: 10 };

/** Install the apps and join the members. */
const JOINED = [
    { action: "publish", fixture: "apps" },
    { action: "install", fixture: "apps" },
    { action: "join", members: "{members}", devices: "{devices}" },
] as const;

/** Create a team. */
const CREATE_TEAM = {
    times: 1,
    step: { call: "team.create", as: "owner", input: { name: "Platform" }, bind: "team" },
    then: [
        {
            spans: { name: "destack.replica.apply", host: "member-*.device-*" },
            atMost: "{members * devices}",
        },
        { observe: { count: "team", on: "member-0.device-0" }, is: 0 },
    ],
} as const;

/** Grant every member the team. */
const GRANT_TEAM = {
    times: "{members}",
    step: {
        call: "team.grant",
        as: "owner",
        input: { id: "$team", relation: "member", subject: "@member-{n}" },
        bind: "grants",
    },
    then: [{ spans: { name: "destack.replica.apply", host: "member-*.device-*" }, atMost: 19 }],
} as const;

/** Create the team's issues. */
const CREATE_ISSUES = {
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
} as const;

/** Comment on the last issue. */
const CREATE_COMMENTS = {
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
} as const;

/** Mark each issue done. */
const UPDATE_ISSUES = {
    times: "{writes}",
    step: { call: "issue.update", as: "member-0", input: { id: "$issue.{n}", state: "done" } },
    then: [
        {
            spans: { name: "destack.replica.apply", host: "member-*.device-*" },
            atMost: "{members * devices}",
        },
    ],
} as const;

/** List the issues. */
const LIST_ISSUES = {
    times: "{writes}",
    step: { call: "issue.list", as: "member-1", input: {} },
    then: [{ spans: { name: "destack.replica.apply", host: "member-*.device-*" }, count: 0 }],
} as const;

/** Revoke the first member's grant. */
const REVOKE_TEAM = {
    times: 1,
    step: { call: "team.revoke", as: "owner", input: { id: "$team", relationshipId: "$grants.0" } },
    then: [{ spans: { name: "destack.replica.apply", host: "member-*.device-*" }, atMost: 19 }],
} as const;

/** Create a team while the members' devices follow. */
export const createTeam = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "createTeam",
    description: t`Create a team while the members' devices follow.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [...JOINED, { operation: "team.create", ...CREATE_TEAM }],
});

/** Grant every member a team. */
export const grantTeam = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "grantTeam",
    description: t`Grant every member a team.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [...JOINED, CREATE_TEAM, { operation: "team.grant", ...GRANT_TEAM }],
});

/** Create a shared team's issues. */
export const createIssue = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "createIssue",
    description: t`Create a shared team's issues.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [...JOINED, CREATE_TEAM, GRANT_TEAM, { operation: "issue.create", ...CREATE_ISSUES }],
});

/** Comment on a shared team's issue. */
export const createComment = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "createComment",
    description: t`Comment on a shared team's issue.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        ...JOINED,
        CREATE_TEAM,
        GRANT_TEAM,
        CREATE_ISSUES,
        { operation: "comment.create", ...CREATE_COMMENTS },
    ],
});

/** Mark a shared team's issues done. */
export const updateIssue = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "updateIssue",
    description: t`Mark a shared team's issues done.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        ...JOINED,
        CREATE_TEAM,
        GRANT_TEAM,
        CREATE_ISSUES,
        { operation: "issue.update", ...UPDATE_ISSUES },
    ],
});

/** List a shared team's issues. */
export const listIssues = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "listIssues",
    description: t`List a shared team's issues.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        ...JOINED,
        CREATE_TEAM,
        GRANT_TEAM,
        CREATE_ISSUES,
        { operation: "issue.list", ...LIST_ISSUES },
    ],
});

/** Revoke a member's grant of a team. */
export const revokeTeam = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "revokeTeam",
    description: t`Revoke a member's grant of a team.`,
    given: { examples: [] },
    parameters: PARAMETERS,
    when: [
        ...JOINED,
        CREATE_TEAM,
        GRANT_TEAM,
        CREATE_ISSUES,
        CREATE_COMMENTS,
        UPDATE_ISSUES,
        LIST_ISSUES,
        { operation: "team.revoke", ...REVOKE_TEAM },
    ],
    then: [
        { observe: { count: "issue" }, is: "{writes}" },
        { observe: { count: "comment" }, is: "{writes}" },
        { observe: { count: "issue", on: "member-1.device-1" }, is: "{writes}" },
        { observe: { count: "issue", on: "member-0.device-0" }, is: 0 },
    ],
});

/** Send bursts of ten messages a second in a channel. */
export const sendMessageBurst = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "sendMessageBurst",
    description: t`Send bursts of ten messages a second in a channel.`,
    given: { examples: [] },
    parameters: { members: 2, devices: 2, burst: 10, bursts: 3 },
    when: [
        ...JOINED,
        { call: "channel.create", as: "owner", input: { name: "general" }, bind: "channel" },
        {
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
        { observe: { count: "message", on: "member-1.device-1" }, is: "{burst * bursts}" },
    ],
});

/** Type into one page with two members at once. */
export const typePage = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "typePage",
    description: t`Type into one page with two members at once.`,
    given: { examples: [] },
    parameters: { members: 2, devices: 2, typists: 2, keys: 10 },
    when: [
        ...JOINED,
        { call: "workspace.create", as: "owner", input: { name: "Docs" }, bind: "workspace" },
        {
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
        { observe: { count: "page", on: "member-1.device-1" }, is: 1 },
    ],
});

/** The members, their devices and the drags. */
const SHAPE_PARAMETERS = { members: 2, devices: 2, moves: 60, drags: 3 };

/** Share a design file with a shape. */
const SHAPE_SHARED = [
    ...JOINED,
    { call: "file.create", as: "owner", input: { name: "Design" }, bind: "file" },
    {
        times: "{members}",
        step: {
            call: "file.grant",
            as: "owner",
            input: { id: "$file", relation: "member", subject: "@member-{n}" },
        },
    },
    { call: "shape.create", as: "owner", input: { parentId: "$file" }, bind: "shape" },
] as const;

/** Drag the shape for a second at 60 moves. */
const DRAG_SHAPE = {
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
    then: [{ spans: { name: "destack.replica.apply", host: "member-*.device-*" }, count: 0 }],
} as const;

/** The shape's end checks. */
const SHAPE_CHECKS = [
    { observe: { count: "shape" }, is: 1 },
    { observe: { count: "shape", on: "member-1.device-1" }, is: 1 },
] as const;

/** Drag a shape for a second at 60 Hz. */
export const dragShape = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "dragShape",
    description: t`Drag a shape for a second at 60 Hz.`,
    given: { examples: [] },
    parameters: SHAPE_PARAMETERS,
    when: [...SHAPE_SHARED, { operation: "shape.drag", ...DRAG_SHAPE }],
    then: SHAPE_CHECKS,
});

/** Drop a dragged shape. */
export const dropShape = defineScenario({
    of: Feed,
    interaction: machineInteraction,
    name: "dropShape",
    description: t`Drop a dragged shape.`,
    given: { examples: [] },
    parameters: SHAPE_PARAMETERS,
    when: [
        ...SHAPE_SHARED,
        DRAG_SHAPE,
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
    then: SHAPE_CHECKS,
});
