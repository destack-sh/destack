import { subscribe, unsubscribe } from "node:diagnostics_channel";
import { cpu } from "@destack/telemetry/meter";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { Traffic } from "../gateway/index.ts";
import { ATTRIBUTION_CHANNEL, Attribution, type Trace } from "./attribution.ts";

/** The space the traces work for. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** The installation the space runs. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-0000000000a2");

/** Trace one invocation taking some CPU time, publishing what it published. */
function trace(cpuTime: number, published: readonly unknown[]): Trace {
    return {
        cpuTime,
        diagnosticsChannelEvents: published.map((message) => ({
            timestamp: 0,
            channel: ATTRIBUTION_CHANNEL,
            message,
        })),
    };
}

test("attribute traced CPU time to the space and installation each invocation published, leaving out unattributed traces", () => {
    // capture what invocations publish, as a trace does
    const published: unknown[] = [];
    const listener = (message: unknown) => published.push(message);
    subscribe(ATTRIBUTION_CHANNEL, listener);
    onTestFinished(() => {
        unsubscribe(ATTRIBUTION_CHANNEL, listener);
    });
    const start = { scope: SPACE, installation: INSTALLATION, secret: "unpublished" };
    Attribution.publish({ scope: SPACE });
    Attribution.publish(Attribution.of(start));
    const [space, installed] = published;

    // trace each fetch, the facet's alarm and the tail's post
    const counts = Attribution.read([
        trace(2, [space]),
        trace(3, [space]),
        trace(5, [installed]),
        trace(1, [installed]),
        trace(7, []),
    ]);

    // count them into the space's traffic as its object does
    const traffic = new Traffic();
    for (const counted of counts.get(SPACE) ?? []) {
        traffic.countTrace(SPACE, counted);
    }
    const taken = traffic.take();

    expect({
        published,
        spaces: [...counts.keys()],
        readings: Traffic.readings(taken),
    }).toEqual({
        published: [{ scope: SPACE }, { scope: SPACE, installation: INSTALLATION }],
        spaces: [SPACE],
        readings: [
            { meter: cpu, scope: SPACE, value: 5 },
            { meter: cpu, scope: SPACE, installation: INSTALLATION, value: 6 },
        ],
    });
});
