import type { Environment, HostCapability, Opener } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";

/** The opener of the real host in tests, refusing to open the person's applications. */
const REFUSING_OPENER: Opener = {
    openUrl: (url) => Promise.reject(new TypeError(`a test opened '${url}' on the host`)),
};

/** Read the real host, for the integration tests that need the operating system itself. */
export function testHost(): Environment<HostCapability> {
    return { ...BunEnvironment.system, opener: REFUSING_OPENER };
}
