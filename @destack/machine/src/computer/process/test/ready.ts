import { BunEnvironment } from "@destack/environment/bun";
import { schema } from "@destack/schema";
import { MachineProcess } from "../process.ts";

/** How long the fixture stays up after it published its endpoint, past its test. */
const LIFETIME_MILLISECONDS = 5000;

/** The host the fixture runs on. */
const environment = BunEnvironment.system;

/** The machine's directory, which the launch passes. */
const directory = schema.string().parse(environment.process.variables["DESTACK_DIRECTORY"]);

/** The endpoint's removal once the fixture ends. */
await using cleanup = new AsyncDisposableStack();

// publish an endpoint in the directory, then stay up a while
await new MachineProcess(directory, environment).publish(cleanup, {
    port: 1,
    token: "0".repeat(64),
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000001"),
    device: schema.identifier("device").parse("device-01996ab0-0000-7000-8000-000000000001"),
});
await environment.clock.sleep(LIFETIME_MILLISECONDS);
