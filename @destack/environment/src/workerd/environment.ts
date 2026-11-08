import { SeverityNumber } from "@opentelemetry/api";
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
    /** The isolate's base, logging to its console and tracing each request's context. */
    get system(): WorkerdEnvironment {
        system ??= {
            ...Environment.system,
            telemetry: Telemetry.local(
                Clock.system,
                (line, severity) => {
                    // write errors as errors, so the Worker's logs keep their level
                    const text = line.trimEnd();
                    if (severity >= SeverityNumber.ERROR) {
                        console.error(text);
                    } else {
                        console.log(text);
                    }
                },
                new AsyncLocalStorageContextManager().enable(),
            ),
        };

        return system;
    },

    /** Write text to the isolate's console, its own log. */
    print(text: string): void {
        console.log(text);
    },
};
