import postgres from "postgres";
import { expect, onTestFinished, test } from "@destack/test";
import { postgresChannel } from "./database.ts";

/** The test server's address, absent where tests run without PostgreSQL. */
const ADDRESS = process.env.DESTACK_TEST_POSTGRES;

/** A message between parties, with a body of some size. */
type Note = { readonly from: string; readonly body: string };

test.skipIf(ADDRESS === undefined)(
    "carry small and large messages whole through a PostgreSQL channel, splitting the large ones",
    async () => {
        // listen with one client and notify with another
        const listener = postgres(ADDRESS!, { max: 1 });
        const sender = postgres(ADDRESS!, { max: 4 });
        const name = `destack_test_${crypto.randomUUID().replaceAll("-", "")}`;
        const received: Note[] = [];
        const joined = Promise.withResolvers<void>();
        const stop = postgresChannel<Note>(listener, name).listen(
            (message) => received.push(message),
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
        const channel = postgresChannel<Note>(sender, name);
        channel.notify({ from: "small", body: "hello" });
        channel.notify({ from: "large", body: large });
        const deadline = Date.now() + 5_000;
        while (received.length < 2 && Date.now() < deadline) {
            await new Promise((resolve) => setTimeout(resolve, 10));
        }

        // receive both exactly, in any order
        expect(received.toSorted((left, right) => left.from.localeCompare(right.from))).toEqual([
            { from: "large", body: large },
            { from: "small", body: "hello" },
        ]);
    },
);
