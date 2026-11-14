import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** Measure a person's first device in a home, a synced object, and a new space. */
export const operationsBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "operations",
    description:
        "a person's first device in a home with an app, an object created and synced, and a space created until served",
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
        "space.create": { action: "create", space: "team-{n}" },
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
        "space.create": {
            "db.client.operation.duration.count": 4527,
            "rpc.server.call.duration.count": 19,
            "rpc.client.call.duration.count": 20,
        },
    },
});
