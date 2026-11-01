import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** The members of the team. */
const MEMBERS = 3;

/** The issues a device syncs first. */
const ISSUES = 50;

/** Measure a tracker's lifecycle: a device's first sync and a wake after a night. */
export const lifecycleBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "lifecycle",
    description: "a new device's first sync of a team's issues and a wake after a night",
    parameters: { members: MEMBERS, issues: ISSUES },
    setup: [
        { action: "install", fixture: "apps" },
        { action: "join", members: MEMBERS, devices: 1 },
        { action: "call", object: "team", method: "create", input: { name: "Platform" } },
        { action: "share", object: "team", relation: "member" },
        {
            action: "call",
            object: "issue",
            method: "create",
            in: "team",
            member: 0,
            input: { title: "Issue {n}" },
            repeat: ISSUES,
        },
    ],
    operations: {
        "device.sync": { action: "join", members: 1, devices: 1, member: 0 },
        "machine.wake": { action: "jump", milliseconds: 8 * 60 * 60_000 },
    },
    iterations: 1,
    limits: {
        "device.sync": {
            "db.client.operation.duration.count": 199,
            "rpc.server.call.duration.count": 7,
            "rpc.client.call.duration.count": 9,
        },
        "machine.wake": {
            "db.client.operation.duration.count": 1146,
            "rpc.server.call.duration.count": 39,
            "rpc.client.call.duration.count": 39,
        },
    },
});
