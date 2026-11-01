import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** Measure the owner's home with three enrolled computers following it. */
export const homeBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "home",
    description: "the owner's home with three enrolled computers following it",
    parameters: { computers: 3 },
    setup: [
        { action: "open", computer: "laptop" },
        { action: "open", computer: "desktop" },
        { action: "open", computer: "tablet" },
    ],
    operations: {
        "role.create": {
            action: "call",
            object: "role",
            method: "create",
            input: {
                name: "role-{n}",
                description: "A role the benchmark creates",
                permissions: [],
            },
        },
        "machine.idle": { action: "run", milliseconds: 60_000 },
        "machine.wake": { action: "jump", milliseconds: 30 * 60_000 },
    },
    iterations: 3,
    limits: {
        "role.create": {
            "db.client.operation.duration.count": 21,
            "rpc.server.call.duration.count": 3,
            "rpc.client.call.duration.count": 3,
        },
        "machine.idle": {
            "db.client.operation.duration.count": 10,
            "rpc.server.call.duration.count": 0,
            "rpc.client.call.duration.count": 0,
        },
        "machine.wake": {
            "db.client.operation.duration.count": 671,
            "rpc.server.call.duration.count": 22,
            "rpc.client.call.duration.count": 22,
        },
    },
});
