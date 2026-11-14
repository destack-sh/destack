import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** Measure a person's first device in a home and an object created and synced to it. */
export const operationsBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "operations",
    description: "a person's first device in a home with an app, and an object created and synced",
    parameters: {},
    setup: [{ action: "install", fixture: "apps" }],
    operations: {
        "person.join": { action: "join", members: 1, devices: 1 },
        "object.create": {
            action: "call",
            object: "team",
            method: "create",
            input: { name: "Platform" },
        },
    },
    iterations: 1,
    limits: {
        "person.join": {
            "db.client.operation.duration.count": 6957,
            "rpc.server.call.duration.count": 30,
            "rpc.client.call.duration.count": 33,
        },
        "object.create": {
            "db.client.operation.duration.count": 31,
            "rpc.server.call.duration.count": 4,
            "rpc.client.call.duration.count": 4,
        },
    },
});
