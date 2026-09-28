import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { RequestFingerprint, RequestId } from "../request/index.ts";
import { defineJournal, Journal } from "./journal.ts";

/** The journal the test records requests in. */
const journal = defineJournal("journal");

/** A request naming a login and its password, which nothing the journal keeps derives from. */
const Login = schema.object({
    name: schema.string(),
    password: schema.sensitive(schema.string()),
    factors: schema.array(schema.object({ code: schema.sensitive(schema.string()).optional() })),
});

test("journal requests by a digest of their input without its sensitive values", async () => {
    const storage = await TestDatabase.create("sqlite", [journal], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const request = { caller: "user-1", scope: "space-1", requestId: RequestId.create() };
    const run = async (value: schema.Infer<typeof Login>) =>
        new Journal(journal).execute(
            database,
            request,
            await RequestFingerprint.hash(Login, value),
            {
                run: async () => "done",
            },
        );

    // keep the digest of the input with its password and codes left out
    const input = { name: "ada", password: "hunter2", factors: [{ code: "123456" }] };
    expect(await run(input)).toBe("done");
    const [stored] = await database.select().from(journal);
    const redacted = new TextEncoder().encode(canonicalize({ name: "ada", factors: [{}] }));
    expect(stored!.digest).toBe(
        new Uint8Array(await crypto.subtle.digest("SHA-256", redacted)).toHex(),
    );

    // replay a retry whatever its sensitive values, and refuse other input under the same identifier
    expect(await run({ ...input, password: "other" })).toBe("done");
    await expect(run({ ...input, name: "bob" })).rejects.toMatchObject({
        code: "CONFLICT",
        message: "request identifier has already been used",
    });
});
