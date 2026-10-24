import { Clock } from "./clock/clock.ts";
import { BunCrypto } from "./crypto/bun.ts";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Environment, type HostCapability } from "./environment/environment.ts";
import { Telemetry } from "./telemetry/telemetry.ts";
import { Workers } from "./worker/worker.ts";
import { BunFileSystem } from "./file/bun.ts";
import { BunHost } from "./host/bun.ts";
import { BunProcess } from "./process/bun.ts";
import { BunNetwork } from "./network/bun.ts";
import { BunServer } from "./server/bun.ts";

/** The system's environment on Bun, built on its first read. */
let system: ReturnType<typeof buildSystem> | undefined;

/** The environment on Bun: the base and the host's capabilities. */
export type BunEnvironment = Environment<HostCapability>;

/** Environments on Bun. */
export const BunEnvironment = {
    /** The system's base and the server capabilities, on the machine Bun runs on. */
    get system(): ReturnType<typeof buildSystem> {
        system ??= buildSystem();

        return system;
    },
};

/** Build the system's environment on Bun, its context manager enabled. */
function buildSystem() {
    return {
        ...Environment.system,
        crypto: BunCrypto,
        network: BunNetwork,
        telemetry: Telemetry.none(Clock.system, new AsyncLocalStorageContextManager().enable()),
        files: new BunFileSystem(),
        process: new BunProcess(),
        server: BunServer,
        host: BunHost.read(),
        workers: Workers.system,
    } satisfies Environment<HostCapability>;
}

export * from "./file/bun.ts";
export * from "./file/lock/lock.ts";
export * from "./host/bun.ts";
export * from "./process/bun.ts";
export * from "./server/bun.ts";
export * from "./crypto/bun.ts";
export * from "./network/bun.ts";
