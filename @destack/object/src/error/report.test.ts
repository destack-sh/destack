import { expect, test } from "@destack/test";
import { testSimulation } from "../test/index.ts";
import { ObjectError } from "./error.ts";
import { refusal } from "./report.ts";

import type {} from "@destack/package/import-meta";

test("set a refusal's Retry-After to its wait in whole seconds, and none for a refusal without one", () => {
    const { telemetry } = testSimulation().environment("test");
    const limited = refusal(
        ObjectError.of("RATE_LIMITED", "limited", { data: { retryAfter: 1500 } }),
        telemetry,
    );
    const missing = refusal(ObjectError.of("SERVER_DOWN", "down"), telemetry);

    // round a wait up to the next second, as a sooner retry would be refused again
    expect([
        [limited.status, limited.headers.get("retry-after")],
        [missing.status, missing.headers.get("retry-after")],
    ]).toEqual([
        [429, "2"],
        [503, null],
    ]);
});
