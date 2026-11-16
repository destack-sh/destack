import { defineScenario } from "@destack/package/declare";
import { objectInteraction } from "../scenario/interaction.ts";
import { trackerExample } from "../test/tracker.ts";
import { ObjectType } from "./object.ts";

/** Rename teams' keys and address them by the old ones, each within its statements. */
export const renameKeys = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "rename-keys",
    description:
        "rename teams' keys and get the teams by their former keys, each verb within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        {
            operation: "team.create",
            times: 20,
            step: { call: "team.create", as: "alice", input: { key: "old{n}" }, bind: "teams" },
        },
        {
            operation: "team.update",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "team.update", as: "alice", input: { id: "$teams.{n}", key: "new{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 14 }],
        },
        {
            operation: "team.get",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "team.get", as: "alice", input: { id: "old{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 3 }],
        },
    ],
    then: [{ observe: { count: "team" }, is: 20 }],
});
