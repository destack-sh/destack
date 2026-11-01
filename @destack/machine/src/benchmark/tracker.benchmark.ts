import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** The members of the tracker's team. */
const MEMBERS = 5;

/** The devices each member follows the team on. */
const DEVICES = 2;

/** Measure a Linear-shaped tracker: a team writing issues and comments, every device following. */
export const trackerBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "tracker",
    description: "a team writing issues and comments while every member's devices follow it",
    parameters: { members: MEMBERS, devices: DEVICES },
    setup: [
        { action: "install", fixture: "apps" },
        { action: "join", members: MEMBERS, devices: DEVICES },
        { action: "call", object: "team", method: "create", input: { name: "Platform" } },
        { action: "share", object: "team", relation: "member" },
    ],
    operations: {
        "issue.create": {
            action: "call",
            object: "issue",
            method: "create",
            in: "team",
            member: 0,
            input: { title: "Issue {n}" },
        },
        "comment.create": {
            action: "call",
            object: "comment",
            method: "create",
            in: "issue",
            member: 1,
            input: { text: "Comment {n}" },
        },
    },
    iterations: 10,
    limits: {
        "issue.create": {
            "db.client.operation.duration.count": 86,
            "rpc.server.call.duration.count": 2.2,
            "rpc.client.call.duration.count": 2.2,
        },
        "comment.create": {
            "db.client.operation.duration.count": 86,
            "rpc.server.call.duration.count": 2.2,
            "rpc.client.call.duration.count": 2.2,
        },
    },
});
