import type { OperatingSystem } from "../host/host.ts";
import { ProcessError } from "../process/error.ts";
import type { Process } from "../process/process.ts";

/** The command that opens a URL in the default application on each operating system. */
const OPENERS: Readonly<Record<OperatingSystem, readonly string[]>> = {
    darwin: ["/usr/bin/open"],
    linux: ["xdg-open"],
    win32: ["rundll32.exe", "url.dll,FileProtocolHandler"],
};

/** The person's default applications on the machine, which open URLs. */
export interface Opener {
    /** Open a URL in its default application, a web URL in the person's browser. */
    readonly openUrl: (url: string) => Promise<void>;
}

/** The opener of a host process, running its operating system's opening command. */
export class ProcessOpener implements Opener {
    /** The process spawning the command. */
    readonly #process: Pick<Process, "spawn">;
    /** The operating system whose command opens URLs. */
    readonly #os: OperatingSystem;

    /** Open URLs with an operating system's command, run as a child of a process. */
    constructor(process: Pick<Process, "spawn">, os: OperatingSystem) {
        this.#process = process;
        this.#os = os;
    }

    /** Open a URL in its default application, refusing a failed command. */
    async openUrl(url: string): Promise<void> {
        // hand the URL to the operating system's command
        const child = this.#process.spawn([...OPENERS[this.#os], url], {
            stdin: "ignore",
            stdout: "ignore",
            stderr: "inherit",
        });

        // refuse a failed command
        const code = await child.exited;
        if (code !== 0) {
            throw ProcessError.of("PROCESS_FAILED", `the opener failed to open ${url}: ${code}`);
        }
    }
}

/** The openers of a simulation's nodes, each recording the URLs it opened, in order. */
export class SimulatedOpener {
    /** The URLs each node opened, by node. */
    readonly #opened = new Map<string, string[]>();

    /** Give a node its opener, recording each URL it opens. */
    of(node: string): Opener {
        return {
            openUrl: async (url) => {
                const opened = this.#opened.get(node) ?? [];
                opened.push(url);
                this.#opened.set(node, opened);
            },
        };
    }

    /** List the URLs a node opened, in order. */
    opened(node: string): readonly string[] {
        return this.#opened.get(node) ?? [];
    }
}
