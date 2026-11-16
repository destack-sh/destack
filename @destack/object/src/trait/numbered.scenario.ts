import { defineScenario } from "@destack/package/declare";
import { objectInteraction } from "../scenario/interaction.ts";
import { trackerExample } from "../test/tracker.ts";
import { ObjectType } from "../object/object.ts";

/** Number issues within a team, and anew in another, each within its statements. */
export const numberIssues = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "number-issues",
    description:
        "create issues numbered within a team and move them to another, each verb within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        {
            operation: "team.create",
            times: 2,
            step: { call: "team.create", as: "alice", input: { key: "T{n}" }, bind: "teams" },
        },
        {
            operation: "issue.create",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "issue.create",
                as: "alice",
                input: { teamId: "$teams.0", title: "issue {n}" },
                bind: "issues",
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 11 }],
        },
        {
            operation: "issue.update",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "issue.update",
                as: "alice",
                input: { id: "$issues.{n}", teamId: "$teams.1" },
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 13 }],
        },
    ],
    then: [{ observe: { count: "issue" }, is: 20 }],
});
