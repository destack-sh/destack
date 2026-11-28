import { expect, testOn } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { SpanPath } from "@destack/environment";
import { testSimulation } from "@destack/test/world";
import { CoalescingVfs } from "./vfs.ts";

/** Test against SQLite alone, as the file tests its own VFS. */
const onSqlite = testOn("sqlite");

onSqlite("keep every commit through coalesced writes", async () => {
    // open a write-ahead database through the coalescing VFS
    const sqlite = await init();
    const vfs = new CoalescingVfs(sqlite, "unix", testSimulation().environment("test").telemetry);
    const database = new sqlite.oo1.DB({ filename: "/coalesced.db", vfs: vfs.name });
    database.exec(
        "PRAGMA locking_mode = EXCLUSIVE; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA wal_autocheckpoint = 20;",
    );
    database.exec("CREATE TABLE note (id INTEGER PRIMARY KEY, title TEXT, body TEXT)");
    database.exec("CREATE INDEX note_title ON note (title)");

    // commit past several checkpoints
    for (let index = 0; index < 120; index += 1) {
        database.exec("BEGIN");
        database.exec({
            sql: "INSERT INTO note (title, body) VALUES (?, ?)",
            bind: [`note ${index}`, "x".repeat(500)],
        });
        database.exec("COMMIT");
    }
    database.close();

    // reopen the file without the coalescing VFS
    const reopened = new sqlite.oo1.DB({ filename: "/coalesced.db", vfs: "unix" });
    reopened.exec("PRAGMA locking_mode = EXCLUSIVE;");
    const read = [
        reopened.selectValue("SELECT count(*) FROM note"),
        reopened.selectValue("SELECT title FROM note ORDER BY id DESC LIMIT 1"),
        reopened.selectValue("PRAGMA integrity_check"),
    ];
    reopened.close();
    expect(read).toEqual([120, "note 119", "ok"]);
});

onSqlite(
    "read back held writes within a transaction, reading other pages without passing them down",
    async () => {
        // open a database whose page cache holds two pages, so reads reach the file
        const sqlite = await init();
        const vfs = new CoalescingVfs(
            sqlite,
            "unix",
            testSimulation().environment("test").telemetry,
        );
        const database = new sqlite.oo1.DB({ filename: "/interleaved.db", vfs: vfs.name });
        database.exec(
            "PRAGMA locking_mode = EXCLUSIVE; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA cache_size = 2;",
        );
        database.exec("CREATE TABLE note (id INTEGER PRIMARY KEY, title TEXT, body TEXT)");

        // insert and read back in each transaction, past several pages
        const counted: number[] = [];
        for (let index = 0; index < 40; index += 1) {
            database.exec("BEGIN");
            database.exec({
                sql: "INSERT INTO note (title, body) VALUES (?, ?)",
                bind: [`note ${index}`, "x".repeat(900)],
            });
            counted.push(
                Number(database.selectValue("SELECT count(*) FROM note WHERE body <> ''")),
            );
            database.exec("COMMIT");
        }
        database.close();

        // reopen the file without the coalescing VFS
        const reopened = new sqlite.oo1.DB({ filename: "/interleaved.db", vfs: "unix" });
        reopened.exec("PRAGMA locking_mode = EXCLUSIVE;");
        const read = [
            reopened.selectValue("SELECT count(*) FROM note"),
            reopened.selectValue("SELECT title FROM note ORDER BY id DESC LIMIT 1"),
            reopened.selectValue("PRAGMA integrity_check"),
        ];
        reopened.close();
        expect([counted, read]).toEqual([
            Array.from({ length: 40 }, (_, index) => index + 1),
            [40, "note 39", "ok"],
        ]);
    },
);

onSqlite("total a commit's writes and syncs below on the running span", async () => {
    // open a write-ahead database as a device does, commit once, then commit two rows in a span
    const sqlite = await init();
    const { telemetry } = testSimulation().environment("test");
    const vfs = new CoalescingVfs(sqlite, "unix", telemetry);
    const database = new sqlite.oo1.DB({ filename: "/totalled.db", vfs: vfs.name });
    database.exec(
        "PRAGMA locking_mode = EXCLUSIVE; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;",
    );
    database.exec("CREATE TABLE note (id INTEGER PRIMARY KEY, title TEXT)");
    database.exec("INSERT INTO note (title) VALUES ('first')");
    const entered = SpanPath.enter(telemetry.context.active(), "commit");
    telemetry.context.with(entered, () =>
        database.exec("BEGIN; INSERT INTO note (title) VALUES ('a'), ('b'); COMMIT;"),
    );
    database.close();

    // expect one coalesced write run and one sync, with their bytes and seconds
    const totals = SpanPath.totals(entered);
    expect({
        writes: totals["destack.storage.write.count"],
        isWritten: Number(totals["destack.storage.write.bytes"]) > 0,
        syncs: totals["destack.storage.sync.count"],
        keys: Object.keys(totals).toSorted(),
    }).toEqual({
        writes: 1,
        isWritten: true,
        syncs: 1,
        keys: [
            "destack.storage.sync.count",
            "destack.storage.sync.duration",
            "destack.storage.write.bytes",
            "destack.storage.write.count",
            "destack.storage.write.duration",
        ],
    });
});
