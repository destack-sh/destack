import { expect, testOn } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { CoalescingVfs } from "./vfs.ts";

/** Test against SQLite alone, as the file tests its own VFS. */
const onSqlite = testOn("sqlite");

onSqlite("keep every commit through coalesced writes", async () => {
    // open a write-ahead database through the coalescing VFS
    const sqlite = await init();
    const vfs = new CoalescingVfs(sqlite, "unix");
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
