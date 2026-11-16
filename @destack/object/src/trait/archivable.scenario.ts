import { defineScenario } from "@destack/package/declare";
import { objectInteraction } from "../scenario/interaction.ts";
import { trackerExample } from "../test/tracker.ts";
import { ObjectType } from "../object/object.ts";

/** Archive issues, list the archive, and unarchive them, each within its statements. */
export const archiveIssues = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "archive-issues",
    description:
        "archive issues, list them from the archive and unarchive them, each verb within its statements",
    given: { examples: [trackerExample] },
    parameters: { calls: 10 },
    when: [
        {
            operation: "team.create",
            times: 1,
            step: { call: "team.create", as: "alice", input: { key: "TRK" }, bind: "teams" },
        },
        {
            operation: "issue.create",
            times: 20,
            step: {
                call: "issue.create",
                as: "alice",
                input: { teamId: "$teams.0", title: "issue {n}" },
                bind: "issues",
            },
        },
        {
            operation: "issue.archive",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "issue.archive", as: "alice", input: { id: "$issues.{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 10 }],
        },
        {
            operation: "issue.list",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "issue.list", as: "alice", input: { archived: "only", limit: 20 } },
            then: [{ metric: { name: "db.client.operation.duration" }, count: 0 }],
        },
        {
            operation: "issue.unarchive",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "issue.unarchive", as: "alice", input: { id: "$issues.{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 9 }],
        },
    ],
    then: [{ observe: { count: "issue" }, is: 20 }],
});
