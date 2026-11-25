import { expect, test } from "@destack/test";
import { testSimulation } from "../test/index.ts";
import { RetryAfter } from "./detail.ts";
import { ObjectError } from "./error.ts";
import { reportError } from "./report.ts";

import type {} from "@destack/package/import-meta";

test("write a wait on headers in whole seconds, and read seconds or a date back in milliseconds", () => {
    // write a wait of one and a half seconds, and data without a wait
    const stated = new Headers();
    const unstated = new Headers();
    RetryAfter.write(stated, { retryAfter: 1500 });
    RetryAfter.write(unstated, { traceId: "a" });

    // round the wait up to two seconds, read a date as the wait until it, and nothing for any other value
    const now = Date.parse("Wed, 21 Oct 2026 07:28:00 GMT");
    const dated = new Headers({ "retry-after": "Wed, 21 Oct 2026 07:28:30 GMT" });
    const passed = new Headers({ "retry-after": "Wed, 21 Oct 2026 07:27:00 GMT" });
    expect([
        stated.get("retry-after"),
        RetryAfter.read(stated, now),
        RetryAfter.read(dated, now),
        RetryAfter.read(passed, now),
        RetryAfter.read(unstated, now),
        RetryAfter.read(new Headers({ "retry-after": "soon" }), now),
        RetryAfter.read(new Headers({ "retry-after": "Mon, 99 Foo 2026 99:99:99 GMT" }), now),
    ]).toEqual(["2", 2000, 30_000, 0, undefined, undefined, undefined]);
});

test("attach the wait a refusal states in its body to its headers, leaving every other answer", async () => {
    // answer a limited call as a procedure does: its call error as JSON, without the header
    const { telemetry } = testSimulation().environment("test");
    const limited = reportError(
        ObjectError.of("RATE_EXHAUSTED", "call rate exceeds its limit", {
            data: { retryAfter: 1500 },
        }),
        telemetry,
    );
    const answers = await Promise.all(
        [
            Response.json(limited.toJSON(), { status: limited.status }),
            Response.json({ text: "hello" }),
            new Response("unavailable", { status: 503 }),
            Response.json(limited.toJSON(), { status: 404 }),
        ].map((response) => RetryAfter.attach(response)),
    );

    // set it on the refusal alone, keeping each answer's status and body
    expect(
        await Promise.all(
            answers.map(async (response) => [
                response.status,
                response.headers.get("retry-after"),
                await response.text(),
            ]),
        ),
    ).toEqual([
        [429, "2", JSON.stringify(limited.toJSON())],
        [200, null, JSON.stringify({ text: "hello" })],
        [503, null, "unavailable"],
        [404, null, JSON.stringify(limited.toJSON())],
    ]);
});

test("send again after each pause a starting server asks for, until the deadline", async () => {
    // expect the served answer within the deadline, and the starting one once the next pause passes it
    expect([await waitUntil(5000), await waitUntil(1500)]).toEqual([
        [200, 3, 2000],
        [503, 2, 1000],
    ]);
});

/** Answer a request as a server starting for its first two attempts, then as serving. */
function starting(attempt: number): Response {
    return attempt < 2
        ? new Response(null, { status: 503, headers: { "retry-after": "1" } })
        : new Response("served");
}

/** Wait on the starting server until a deadline, reading the answer, attempts and time passed. */
async function waitUntil(deadline: number): Promise<number[]> {
    // advance the clock by each pause
    let now = 0;
    let attempts = 0;
    const clock = {
        now: () => now,
        sleep: async (milliseconds: number) => void (now += milliseconds),
    };
    const response = await RetryAfter.wait(async () => starting(attempts++), clock, deadline);

    return [response.status, attempts, now];
}
