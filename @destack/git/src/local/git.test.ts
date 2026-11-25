import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { testSimulation } from "@destack/test/world";
import { SpanRunner } from "@destack/environment";
import { Git } from "./git.ts";

test("send the active trace after a caller's headers", async () => {
    // make a repository on the host's processes, on a simulated telemetry
    await using simulation = testSimulation();
    const host = testHost();
    const { telemetry } = simulation.environment("client");
    const environment = { ...host, telemetry };
    const directory = await host.files.makeTemporaryDirectory("git-trace-");
    onTestFinished(() => host.files.remove(directory, { recursive: true }));
    await Git.run(directory, ["init", "--quiet"], environment);

    // read the extra headers git is configured with beside a lease's header, outside a span
    const lease = { configuration: [["http.extraHeader", "Authorization: Bearer lease"]] as const };
    const read = () =>
        Git.run(directory, ["config", "--get-all", "http.extraHeader"], environment, lease);
    const outside = await read();

    // read them again inside a span
    const run = SpanRunner.of(
        telemetry.tracers.getTracer("test"),
        telemetry.meters.getMeter("test"),
        telemetry,
    );
    const [inside, { traceId, spanId }] = await run(
        "git.push",
        {},
        async (span) => [await read(), span.spanContext()] as const,
    );

    // expect the lease's header alone outside a span, and the span's context after it inside one
    expect([outside, inside]).toEqual([
        "Authorization: Bearer lease\n",
        `Authorization: Bearer lease\ntraceparent: 00-${traceId}-${spanId}-01\n`,
    ]);
});
