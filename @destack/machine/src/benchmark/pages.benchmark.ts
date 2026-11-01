import { defineBenchmark } from "@destack/package/declare";
import { Computer } from "../computer/computer.ts";
import { worldInteraction } from "./world.ts";

/** The members of the workspace. */
const MEMBERS = 5;

/** The devices each member follows the workspace on. */
const DEVICES = 2;

/** The members typing into one page at once. */
const TYPISTS = 3;

/** Measure a Notion-shaped workspace: members typing into one page, every device following. */
export const pagesBenchmark = defineBenchmark({
    of: Computer,
    interaction: worldInteraction,
    name: "pages",
    description: "members typing into one page at once while every member's devices follow it",
    parameters: { members: MEMBERS, devices: DEVICES, typists: TYPISTS },
    setup: [
        { action: "install", fixture: "apps" },
        { action: "join", members: MEMBERS, devices: DEVICES },
        { action: "call", object: "workspace", method: "create", input: { name: "Docs" } },
        { action: "share", object: "workspace", relation: "member" },
        {
            action: "call",
            object: "page",
            method: "create",
            in: "workspace",
            input: { title: "Meeting notes" },
        },
    ],
    operations: {
        "page.type": {
            action: "type",
            object: "page",
            field: "body",
            text: "a",
            members: TYPISTS,
        },
    },
    iterations: 10,
    limits: {
        "page.type": {
            "db.client.operation.duration.count": 483,
            "rpc.server.call.duration.count": 8,
            "rpc.client.call.duration.count": 8,
        },
    },
});
