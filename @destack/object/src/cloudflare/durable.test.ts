import { testSimulation } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { DurableObjectWorkload } from "./durable.ts";

test("start the workload on the object's alarm before any event, serve requests and run alarms through it for at most ten minutes", async () => {
    // record the object's alarm and its events in order
    const events: string[] = [];
    const storage = {
        setAlarm: async (at: number) => {
            events.push(`alarm at ${at}`);
        },
        deleteAlarm: async () => {
            events.push("alarm deleted");
        },
    };
    const started = Promise.withResolvers<void>();
    const deadlines: number[] = [];
    const { clock } = testSimulation().environment("test");

    // start a workload that sets the alarm and serves once the start settles
    const object = new DurableObjectWorkload(
        {
            storage,
            blockConcurrencyWhile: async (closure) => {
                events.push("blocked");
                const value = await closure();
                events.push("unblocked");

                return value;
            },
        },
        async (alarm) => {
            await alarm.setAlarm(1000);
            await started.promise;

            return {
                fetch: async (request) => new Response(new URL(request.url).pathname),
                alarm: async (deadline) => {
                    events.push("alarm");
                    deadlines.push(deadline);
                },
            };
        },
        clock,
    );

    // delay the first request until the start settles before serving it and an alarm
    const answering = object.fetch(new Request("https://object.test/served"));
    events.push("requested");
    started.resolve();
    const answer = await answering;
    await object.alarm();
    const at = clock.now();

    // bound the alarm ten minutes after it rang
    const [deadline] = deadlines;
    expect({
        text: await answer.text(),
        events,
        deadline,
    }).toEqual({
        text: "/served",
        events: ["blocked", "alarm at 1000", "requested", "unblocked", "alarm"],
        deadline: at + 600_000,
    });
});
