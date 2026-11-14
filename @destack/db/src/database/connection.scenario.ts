import { defineScenario } from "@destack/package/declare";
import { databaseInteraction } from "../interaction.ts";
import { DatabaseConnection } from "./connection.ts";

/** Insert, read and update notes among a thousand, and insert them in log transactions. */
export const insertUpdateAndTransact = defineScenario({
    of: DatabaseConnection,
    interaction: databaseInteraction,
    name: "insert-update-and-transact",
    description:
        "insert, read and update notes among a thousand, and insert them in log transactions",
    given: { examples: [] },
    parameters: { notes: 1000, times: 100 },
    when: [
        { action: "insert", rows: "{notes}" },
        {
            operation: "note.insert",
            times: "{times}",
            step: { action: "insert", rows: 1 },
            then: [
                { metric: { name: "db.client.operation.duration" }, atMost: 2 },
                { metric: { name: "destack.transaction.run.duration" }, atMost: 1 },
            ],
        },
        {
            operation: "note.read",
            times: "{times}",
            step: { action: "read" },
            then: [{ metric: { name: "db.client.operation.duration" }, atMost: 1 }],
        },
        {
            operation: "note.update",
            times: "{times}",
            step: { action: "update" },
            then: [
                { metric: { name: "db.client.operation.duration" }, atMost: 2 },
                { metric: { name: "destack.transaction.run.duration" }, atMost: 1 },
            ],
        },
        {
            operation: "note.transact",
            times: "{times}",
            step: { action: "transact" },
            then: [
                { metric: { name: "db.client.operation.duration" }, atMost: 6 },
                { metric: { name: "destack.transaction.run.duration" }, atMost: 2 },
            ],
        },
    ],
    then: [{ observe: { count: "notes" }, is: "{notes + times + times}" }],
});
