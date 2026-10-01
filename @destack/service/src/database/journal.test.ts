import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { RequestId } from "../request/index.ts";
import { defineJournal, Journal } from "./journal.ts";

/** The journal the test records requests in. */
const journal = defineJournal("journal");

/** A login request with a password. */
const Login = schema.object({
    name: schema.string(),
    password: schema.sensitive(schema.string()),
    factors: schema.array(schema.object({ code: schema.sensitive(schema.string()).optional() })),
});

test("journal requests by a digest no reader can recompute, replay retries on every instance, and refuse a changed value", async () => {
    const storage = await TestDatabase.create("sqlite", [journal], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const request = { caller: "user-1", scope: "space-1", requestId: RequestId.create() };
    // run requests on two instances sharing the deployment's key
    const key = await Journal.importKey(crypto.getRandomValues(new Uint8Array(32)));
    const instances = [
        new Journal(journal, async () => key),
        new Journal(journal, async () => key),
    ];
    const run = async (value: schema.Infer<typeof Login>, instance = instances[0]!) => {
        const sensitive: unknown[] = [];
        const redacted = schema.redact(Login, value, (found) => sensitive.push(found));
        const fingerprint = await instance.fingerprint(redacted, sensitive);

        return instance.execute(database, request, fingerprint, { run: async () => "done" });
    };
    const refused = (value: schema.Infer<typeof Login>) =>
        run(value).then(
            () => "replayed",
            (error: { code: string; message: string }) => [error.code, error.message],
        );

    // run the request, and replay a retry of the same input on the other instance
    const input = { name: "ada", password: "hunter2", factors: [{ code: "123456" }] };
    const first = await run(input);
    const replayed = await run(input, instances[1]);

    // refuse a retry changing a sensitive or a plain value
    const conflict = ["CONFLICT", "request identifier has already been used"];
    const changed = [
        await refused({ ...input, password: "other" }),
        await refused({ ...input, name: "bob" }),
    ];

    // keep a digest that neither the redacted nor the whole input recomputes
    const sha = async (value: unknown) =>
        new Uint8Array(
            await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonicalize(value))),
        ).toHex();
    const [stored] = await database.select().from(journal);
    const recomputed = [
        stored!.digest === (await sha({ name: "ada", factors: [{}] })),
        stored!.digest === (await sha(input)),
    ];
    expect([first, replayed, changed, recomputed]).toEqual([
        "done",
        "done",
        [conflict, conflict],
        [false, false],
    ]);
});
