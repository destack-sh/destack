import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { Outcome } from "./provider.ts";

/** The time every answer is read at. */
const NOW = Date.UTC(2026, 9, 5);

/** The origin the simulated endpoint listens at. */
const ORIGIN = "https://hooks.example.com";

test("read an endpoint's answers as sent, gone, retried after their wait, or rejected", () => {
    // answer each status once, and throttles with a valid, empty, negative and impossible wait
    const answers = [
        new Response(null, { status: 204 }),
        new Response(null, { status: 410, statusText: "Gone" }),
        new Response(null, { status: 429, headers: { "Retry-After": "30" } }),
        new Response(null, { status: 429, headers: { "Retry-After": "" } }),
        new Response(null, { status: 429, headers: { "Retry-After": "-5" } }),
        new Response(null, {
            status: 429,
            headers: { "Retry-After": "Mon, 99 Foo 2026 99:99:99 GMT" },
        }),
        new Response(null, { status: 503 }),
        new Response(null, { status: 408 }),
        new Response(null, { status: 400, statusText: "Bad Request" }),
    ];
    expect(answers.map((response) => Outcome.read(response, NOW))).toEqual([
        { kind: "sent" },
        { kind: "failed", refusal: { reason: "RECIPIENT_GONE", code: "410", message: "Gone" } },
        {
            kind: "deferred",
            after: 30_000,
            refusal: { reason: "PROVIDER_THROTTLED", code: "429", message: "" },
        },
        { kind: "deferred", refusal: { reason: "PROVIDER_THROTTLED", code: "429", message: "" } },
        { kind: "deferred", refusal: { reason: "PROVIDER_THROTTLED", code: "429", message: "" } },
        { kind: "deferred", refusal: { reason: "PROVIDER_THROTTLED", code: "429", message: "" } },
        { kind: "deferred", refusal: { reason: "PROVIDER_UNAVAILABLE", code: "503", message: "" } },
        { kind: "deferred", refusal: { reason: "PROVIDER_TIMEOUT", code: "408", message: "" } },
        {
            kind: "failed",
            refusal: { reason: "MESSAGE_REJECTED", code: "400", message: "Bad Request" },
        },
    ]);
});

test("retry a post the network failed to deliver or its timeout ended, and rethrow any other failure", async () => {
    // post to an origin nothing serves, and to one that never answers before the timeout
    const simulation = new Simulation({ seed: 1, start: NOW });
    const { clock } = simulation;
    const network = simulation.network.of("sender");
    const unserved = await Outcome.post(network.fetch, new Request(`${ORIGIN}/a`), clock);
    const silent = new AbortController();
    silent.abort(new DOMException("the post timed out", "TimeoutError"));
    const timedOut = await Outcome.post(
        (request) => Promise.reject(request.signal.reason),
        new Request(`${ORIGIN}/b`, { signal: silent.signal }),
        clock,
    );

    // rethrow a failure that is no network failure
    const bug = new RangeError("a bug in the sender");
    const failing = await Outcome.post(
        () => Promise.reject(bug),
        new Request(`${ORIGIN}/c`),
        clock,
    ).catch((error: unknown) => error);
    expect({ unserved, timedOut, failing }).toEqual({
        unserved: {
            kind: "deferred",
            refusal: {
                reason: "PROVIDER_UNREACHABLE",
                message: `fetch failed: nothing serves ${ORIGIN}`,
            },
        },
        timedOut: {
            kind: "deferred",
            refusal: { reason: "PROVIDER_TIMEOUT", message: "the post timed out" },
        },
        failing: bug,
    });
});
