import { defineScenario } from "@destack/package/declare";
import { objectInteraction } from "../scenario/interaction.ts";
import { notesExample } from "../test/note.ts";
import { ObjectType } from "./object.ts";

/** Run each verb of a type as one person, each within its statements on every database. */
export const runEachVerb = defineScenario({
    of: ObjectType,
    interaction: objectInteraction,
    name: "run-each-verb",
    description:
        "create, get, list, update, nest and delete notebooks as one person, each verb within its statements",
    given: { examples: [notesExample] },
    parameters: { calls: 10 },
    when: [
        {
            operation: "notebook.create",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "notebook.create",
                as: "alice",
                input: { name: "book {n}" },
                bind: "books",
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 12 }],
        },
        {
            operation: "notebook.get",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "notebook.get", as: "alice", input: { id: "$books.{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 2 }],
        },
        {
            operation: "notebook.list",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "notebook.list", as: "alice", input: { limit: 20 } },
            then: [{ metric: { name: "db.client.operation.duration" }, count: 0 }],
        },
        {
            operation: "notebook.update",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "notebook.update",
                as: "alice",
                input: { id: "$books.{n}", name: "renamed {n}" },
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 12 }],
        },
        {
            operation: "note.create",
            times: "{calls}",
            warmup: "{calls}",
            step: {
                call: "note.create",
                as: "alice",
                input: { parentId: "$books.{n}", title: "note {n}" },
            },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 13 }],
        },
        {
            operation: "notebook.delete",
            times: "{calls}",
            warmup: "{calls}",
            step: { call: "notebook.delete", as: "alice", input: { id: "$books.{n}" } },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 15 }],
        },
    ],
    then: [{ observe: { count: "notebook" }, is: 0 }],
});
