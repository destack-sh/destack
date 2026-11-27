import type { Environment, HostCapability, Opener } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { LineExporter, LOG_LEVEL_VARIABLE } from "@destack/environment/line";

/** The level a test run prints its host's telemetry at unless it asks for more: warnings. */
const TEST_LOG_LEVEL = "warn";

/** The opener of the real host in tests, refusing to open the person's applications. */
const REFUSING_OPENER: Opener = {
    openUrl: (url) => Promise.reject(new TypeError(`a test opened '${url}' on the host`)),
};

/** Read the real host, for the integration tests that need the operating system itself. */
export function testHost(): Environment<HostCapability> {
    // print the host's telemetry as every process does, at warnings unless the run asks for more
    const system = BunEnvironment.system;
    const variables = { [LOG_LEVEL_VARIABLE]: TEST_LOG_LEVEL, ...system.process.variables };
    const exporter = new LineExporter(
        (line, severity) => LineExporter.console(line, severity),
        LineExporter.options(variables),
    );
    const telemetry = exporter.telemetry(system.clock, system.telemetry.context);

    return { ...system, telemetry, opener: REFUSING_OPENER };
}
