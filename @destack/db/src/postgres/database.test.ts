import postgres from "postgres";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { typedChannel } from "../channel/channel.ts";
import { postgresChannel } from "./database.ts";

/** The test server's address, absent where tests run without PostgreSQL. */
const ADDRESS = process.env["DESTACK_TEST_POSTGRES"];

/** A message between parties, with a body of some size. */
const Note = schema.object({ from: schema.string(), body: schema.string() });
/** A message between parties, with a body of some size. */
type Note = schema.Infer<typeof Note>;

test.skipIf(ADDRESS === undefined)(
    "carry small and large messages whole through a PostgreSQL channel, splitting the large ones",
    async () => {
        // listen with one client and notify with another
        if (ADDRESS === undefined) {
            throw new Error("the test server has no address");
        }
        const listener = postgres(ADDRESS, { max: 1 });
        const sender = postgres(ADDRESS, { max: 4 });
        const name = `destack_test_${crypto.randomUUID().replaceAll("-", "")}`;
        const received: Note[] = [];
        const joined = Promise.withResolvers<void>();
        const isReceived = Promise.withResolvers<void>();
        const stop = typedChannel(postgresChannel(listener, name), Note).listen(
            (message) => {
                received.push(message);
                if (received.length === 2) {
                    isReceived.resolve();
                }
            },
            () => joined.resolve(),
        );
        onTestFinished(async () => {
            // stop listening before closing both clients
            stop();
            await Promise.all([listener.end(), sender.end()]);
        });
        await joined.promise;

        // send a small message and one of about 60 kB with multi-byte characters across fragments
        const large = "äöü€😀".repeat(5_000);
        const channel = typedChannel(postgresChannel(sender, name), Note);
        channel.notify({ from: "small", body: "hello" });
        channel.notify({ from: "large", body: large });
        await isReceived.promise;

        // receive both exactly, in any order
        expect(received.toSorted((left, right) => left.from.localeCompare(right.from))).toEqual([
            { from: "large", body: large },
            { from: "small", body: "hello" },
        ]);
    },
);
