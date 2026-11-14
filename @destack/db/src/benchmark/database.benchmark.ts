import { defineBenchmark } from "@destack/package/declare";
import { DatabaseConnection } from "../database/connection.ts";
import { databaseInteraction } from "./interaction.ts";

/** The notes the database keeps before the measured operations. */
const NOTES = 1000;

/** The bytes of each stored blob: 64 KiB, a typical build module. */
const BLOB_BYTES = 65_536;

/** Measure a database's key operations: writes and reads of rows, log transactions and blobs. */
export const databaseBenchmark = defineBenchmark({
    of: DatabaseConnection,
    interaction: databaseInteraction,
    name: "database",
    description: "a database's row writes and reads, log transactions, and blob writes and reads",
    parameters: { notes: NOTES },
    setup: [{ action: "insert", rows: NOTES }],
    operations: {
        "note.insert": { action: "insert", rows: 1 },
        "note.read": { action: "read" },
        "note.update": { action: "update" },
        "note.transact": { action: "transact" },
        "blob.write": { action: "write", bytes: BLOB_BYTES },
        "blob.fetch": { action: "fetch" },
    },
    iterations: 100,
    limits: {
        "note.insert": {
            "db.client.operation.duration.count": 2,
            "destack.transaction.run.duration.count": 1,
        },
        "note.read": { "db.client.operation.duration.count": 1 },
        "note.update": {
            "db.client.operation.duration.count": 2,
            "destack.transaction.run.duration.count": 1,
        },
        "note.transact": {
            "db.client.operation.duration.count": 6,
            "destack.transaction.run.duration.count": 2,
        },
        "blob.write": {
            "db.client.operation.duration.count": 4,
            "destack.transaction.run.duration.count": 2,
        },
        "blob.fetch": { "db.client.operation.duration.count": 1 },
    },
});
