import { Clock } from "../clock/clock.ts";
import { Compression } from "../compression/compression.ts";
import { Crypto } from "../crypto/crypto.ts";
import type { FileSystem } from "../file/file.ts";
import { Network } from "../network/network.ts";
import { HttpTransport } from "../network/transport.ts";
import { HttpServer } from "../server/http.ts";
import { InstrumentedFiles } from "../file/instrumented.ts";
import { InstrumentedProcess } from "../process/instrumented.ts";
import type { Host } from "../host/host.ts";
import type { Opener } from "../opener/opener.ts";
import type { Process } from "../process/process.ts";
import { Random } from "../random/random.ts";
import { Scheduler } from "../scheduler/scheduler.ts";
import type { Server } from "../server/server.ts";
import { Telemetry } from "../telemetry/telemetry.ts";
import type { WorkerHost } from "../worker/worker.ts";

/** The effects some targets have beside the base, each requested by name. */
export interface Capability {
    /** The files. */
    readonly files: FileSystem;
    /** The running process and its children. */
    readonly process: Process;
    /** The accepting of connections. */
    readonly server: Server;
    /** The machine. */
    readonly host: Host;
    /** The person's default applications, which open URLs. */
    readonly opener: Opener;
    /** The starting of worker threads. */
    readonly workers: WorkerHost;
}

/** The capabilities a host process has beside the base. */
export type HostCapability = "files" | "process" | "server" | "host" | "opener" | "workers";

/** The effects code runs on: the base every target has, and the capabilities it requests. */
export type Environment<Name extends keyof Capability = never> = {
    /** The time. */
    readonly clock: Clock;
    /** The scheduler its work runs under. */
    readonly scheduler: Scheduler;
    /** The randomness. */
    readonly random: Random;
    /** The WebCrypto operations. */
    readonly crypto: Crypto;
    /** The network. */
    readonly network: Network;
    /** The compression streams. */
    readonly compression: Compression;
    /** The telemetry it emits through. */
    readonly telemetry: Telemetry;
    /** The BCP 47 locale it formats text in, such as en-US. */
    readonly locale: string;
    /** The IANA time zone it formats times in, such as Europe/Vienna. */
    readonly timeZone: string;
} & Pick<Capability, Name>;

/** The system's base environment, built on its first read. */
let system: Environment | undefined;

/** Environments. */
export const Environment = {
    /** The system's base: its clock, randomness, WebCrypto and network, on every target. */
    get system(): Environment {
        // read the host's locale and time zone, and build the providers, once
        if (system === undefined) {
            const { locale, timeZone } = Intl.DateTimeFormat().resolvedOptions();
            const telemetry = Telemetry.none(Clock.system);
            system = Environment.instrument(
                {
                    clock: Clock.system,
                    scheduler: Scheduler.system,
                    random: Random.system,
                    crypto: Crypto.system,
                    network: Network.system,
                    compression: Compression.system,
                    telemetry,
                    locale,
                    timeZone,
                },
                telemetry,
            );
        }

        return system;
    },

    /** Instrument an environment's boundaries on a telemetry: network, server, files, process. */
    instrument<Effects extends Partial<Environment<"files" | "process" | "server">> & Environment>(
        environment: Effects,
        telemetry: Telemetry,
    ): Effects {
        // wrap each boundary the environment has, unwrapping an instrumented one first
        const network = new HttpTransport(HttpTransport.sourceOf(environment.network), telemetry);
        const server =
            environment.server === undefined
                ? {}
                : { server: new HttpServer(HttpServer.sourceOf(environment.server), telemetry) };
        const files =
            environment.files === undefined
                ? {}
                : {
                      files: new InstrumentedFiles(
                          InstrumentedFiles.sourceOf(environment.files),
                          telemetry,
                      ),
                  };
        const process =
            environment.process === undefined
                ? {}
                : {
                      process: new InstrumentedProcess(
                          InstrumentedProcess.sourceOf(environment.process),
                          telemetry,
                      ),
                  };

        return { ...environment, telemetry, network, ...server, ...files, ...process };
    },
};
