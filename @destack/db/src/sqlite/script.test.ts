import { expect, test } from "@destack/test";
import { SqliteScript } from "./script.ts";

test("split a script into statements, keeping trigger bodies, strings and comments whole", () => {
    const script = `
        CREATE TABLE note (id TEXT PRIMARY KEY, title TEXT DEFAULT 'a;b');
        -- a comment; with a semicolon
        CREATE TEMP TRIGGER note_log AFTER INSERT ON note BEGIN
            INSERT INTO log (row) VALUES (json_object('id', NEW.id));
            UPDATE counts SET n = CASE WHEN n IS NULL THEN 1 ELSE n + 1 END;
        END;
        /* block; comment */ INSERT INTO "odd;name" VALUES ('it''s;'); SELECT 1`;

    // close each statement at the semicolon completing it, and keep the last one without its semicolon
    expect(SqliteScript.statements(script)).toEqual([
        "CREATE TABLE note (id TEXT PRIMARY KEY, title TEXT DEFAULT 'a;b');",
        `-- a comment; with a semicolon
        CREATE TEMP TRIGGER note_log AFTER INSERT ON note BEGIN
            INSERT INTO log (row) VALUES (json_object('id', NEW.id));
            UPDATE counts SET n = CASE WHEN n IS NULL THEN 1 ELSE n + 1 END;
        END;`,
        `/* block; comment */ INSERT INTO "odd;name" VALUES ('it''s;');`,
        "SELECT 1",
    ]);
});
