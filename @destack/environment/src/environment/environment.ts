import { Clock } from "../clock/clock.ts";
import { Compression } from "../compression/compression.ts";
import { Crypto } from "../crypto/crypto.ts";
import type { FileSystem } from "../file/file.ts";
import { Network } from "../network/network.ts";
import type { Host } from "../host/host.ts";
import type { Process } from "../process/process.ts";
import { Random } from "../random/random.ts";
import type { Server } from "../server/server.ts";
import { Telemetry } from "../telemetry/telemetry.ts";
import type { Workers } from "../worker/worker.ts";

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
    /** The starting of worker threads. */
    readonly workers: Workers;
}

/** The capabilities a host process has beside the base. */
export type HostCapability = "files" | "process" | "server" | "host" | "workers";

/** The effects code runs on: the base every target has, and the capabilities it requests. */
export type Environment<Name extends keyof Capability = never> = {
    /** The time. */
    readonly clock: Clock;
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
            system = {
                clock: Clock.system,
                random: Random.system,
                crypto: Crypto.system,
                network: Network.system,
                compression: Compression.system,
                telemetry: Telemetry.none(Clock.system),
                locale,
                timeZone,
            };
        }

        return system;
    },
};
