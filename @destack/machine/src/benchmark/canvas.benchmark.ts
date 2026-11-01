import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** The members of the file. */
const MEMBERS = 5;

/** The devices each member follows the file on. */
const DEVICES = 2;

/** The moves of one drag: a second at 60 Hz. */
const MOVES = 60;

/** Measure a Figma-shaped file: a member drags a shape at 60 Hz, then drops it. */
export const canvasBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "canvas",
    description:
        "a member dragging a shape for a second, broadcasting its moves at 60 Hz, then dropping it, while every member's devices follow it",
    parameters: { members: MEMBERS, devices: DEVICES, moves: MOVES },
    setup: [
        { action: "install", fixture: "apps" },
        { action: "join", members: MEMBERS, devices: DEVICES },
        { action: "call", object: "file", method: "create", input: { name: "Design" } },
        { action: "share", object: "file", relation: "member" },
        { action: "call", object: "shape", method: "create", in: "file", input: {} },
    ],
    operations: {
        "shape.drag": {
            action: "broadcast",
            object: "shape",
            member: 0,
            event: { x: "{n}", y: "{n}" },
            repeat: MOVES,
            rate: MOVES,
        },
        "shape.drop": {
            action: "call",
            object: "shape",
            method: "update",
            on: "shape",
            member: 0,
            input: { x: "{n}", y: "{n}" },
        },
    },
    iterations: 3,
    limits: {
        "shape.drag": {
            "db.client.operation.duration.count": 62,
            "rpc.server.call.duration.count": 120,
            "rpc.client.call.duration.count": 120,
        },
        "shape.drop": {
            "db.client.operation.duration.count": 83,
            "rpc.server.call.duration.count": 3,
            "rpc.client.call.duration.count": 3,
        },
    },
});
