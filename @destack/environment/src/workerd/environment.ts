import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Clock } from "../clock/clock.ts";
import { Environment } from "../environment/environment.ts";
import { Telemetry } from "../telemetry/telemetry.ts";

/** The system's environment on workerd, built on its first read. */
let system: WorkerdEnvironment | undefined;

/** The environment on workerd: the base, which an isolate serving fetches has alone. */
export type WorkerdEnvironment = Environment;

/** Environments on workerd. */
export const WorkerdEnvironment = {
    /** The isolate's base, its telemetry context following each request through async hooks. */
    get system(): WorkerdEnvironment {
        system ??= {
            ...Environment.system,
            telemetry: Telemetry.none(Clock.system, new AsyncLocalStorageContextManager().enable()),
        };

        return system;
    },
};
