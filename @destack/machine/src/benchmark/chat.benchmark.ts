import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** The members of the channel. */
const MEMBERS = 5;

/** The devices each member follows the channel on. */
const DEVICES = 2;

/** The messages of one burst. */
const BURST = 10;

/** Measure a Slack-shaped channel: bursts of messages, every device following. */
export const chatBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "chat",
    description: "bursts of messages in a channel while every member's devices follow it",
    parameters: { members: MEMBERS, devices: DEVICES, burst: BURST },
    setup: [
        { action: "install", fixture: "apps" },
        { action: "join", members: MEMBERS, devices: DEVICES },
        { action: "call", object: "channel", method: "create", input: { name: "general" } },
        { action: "share", object: "channel", relation: "member" },
    ],
    operations: {
        "message.burst": {
            action: "call",
            object: "message",
            method: "create",
            in: "channel",
            member: 1,
            input: { text: "Message {n}" },
            repeat: BURST,
            rate: BURST,
        },
    },
    iterations: 3,
    limits: {
        "message.burst": {
            "db.client.operation.duration.count": 880,
            "rpc.server.call.duration.count": 21,
            "rpc.client.call.duration.count": 21,
        },
    },
});
