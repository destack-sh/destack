import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
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

test.each(TEST_DIALECTS)(
    "answer two concurrent copies of a request with the one outcome that commits on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [journal], { isMigrated: true });
        onTestFinished(() => storage.close());
        const instance = new Journal(journal, async () => {
            throw new TypeError("no sensitive input");
        });
        const request = { caller: "user-1", scope: "space-1", requestId: RequestId.create() };
        const fingerprint = await instance.fingerprint({ name: "ada" }, []);

        // let both copies run only once both are inside their transactions
        let entered = 0;
        let release: () => void = () => {};
        const both = new Promise<void>((resolve) => (release = resolve));
        const copy = () =>
            instance
                .execute(storage.database, request, fingerprint, {
                    run: async () => {
                        entered += 1;
                        if (entered === 2) {
                            release();
                        }
                        await Promise.race([
                            both,
                            new Promise((resolve) => setTimeout(resolve, 200)),
                        ]);

                        return "done";
                    },
                })
                .then(
                    (value) => ["value", value],
                    (error: { code: string; message: string }) => [error.code, error.message],
                );
        // answer both copies with the committed outcome, recorded once
        const answers = await Promise.all([copy(), copy()]);
        const rows = await storage.database.select().from(journal);
        expect([answers, rows.length]).toEqual([
            [
                ["value", "done"],
                ["value", "done"],
            ],
            1,
        ]);
    },
);
