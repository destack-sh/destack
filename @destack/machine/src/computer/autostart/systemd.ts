import { join } from "node:path";
import type { AutostartRegistration } from "./autostart.ts";
import { Command, type CommandHost, SERVICE_MILLISECONDS } from "../process/command.ts";

/** The permissions of the unit file: the person's alone. */
const REGISTRATION_MODE = 0o600;

/** A systemd user service running the machine in the foreground. */
export class SystemdService implements AutostartRegistration {
    /** The unit name without its suffix. */
    readonly name: string;
    /** The machine state directory. */
    readonly directory: string;
    /** The unit file in the user's systemd configuration directory. */
    readonly path: string;
    /** Whether systemd supervises the running machine: it does. */
    readonly isSupervised = true;
    /** The host systemd runs on. */
    readonly #environment: CommandHost;

    /** Locate the service of a unit name and machine directory on a host. */
    constructor(name: string, directory: string, environment: CommandHost) {
        // set the unit and directory, and the unit file in the person's configuration directory
        this.name = name;
        this.directory = directory;
        this.#environment = environment;
        const configuration =
            environment.process.variables["XDG_CONFIG_HOME"] ??
            join(environment.host.home, ".config");
        this.path = join(configuration, "systemd/user", `${name}.service`);
    }

    /** Publish the complete unit atomically through a temporary file, and enable it. */
    async install(executable: string, temporary: string): Promise<void> {
        // publish, load and enable the unit
        const { files } = this.#environment;
        await files.writeTextFile(temporary, this.describe(executable), {
            createNew: true,
            mode: REGISTRATION_MODE,
        });
        await files.rename(temporary, this.path);
        await Command.run(this.#environment, ["systemctl", "--user", "daemon-reload"], {
            timeout: SERVICE_MILLISECONDS,
        });
        await Command.run(
            this.#environment,
            ["systemctl", "--user", "enable", `${this.name}.service`],
            { timeout: SERVICE_MILLISECONDS },
        );
    }

    /** Let systemd retain and supervise the foreground process. */
    async start(): Promise<void> {
        await Command.run(
            this.#environment,
            ["systemctl", "--user", "start", `${this.name}.service`],
            { timeout: SERVICE_MILLISECONDS },
        );
    }

    /** Stop the service before replacing the selected release. */
    async stop(): Promise<void> {
        await Command.run(
            this.#environment,
            ["systemctl", "--user", "stop", `${this.name}.service`],
            { timeout: SERVICE_MILLISECONDS },
        );
    }

    /** Disable and remove the stopped service's unit. */
    async uninstall(): Promise<void> {
        await Command.run(
            this.#environment,
            ["systemctl", "--user", "disable", `${this.name}.service`],
            { timeout: SERVICE_MILLISECONDS },
        );
        await this.#environment.files.remove(this.path);
        await Command.run(this.#environment, ["systemctl", "--user", "daemon-reload"], {
            timeout: SERVICE_MILLISECONDS,
        });
    }

    /** Report whether the registered unit runs an executable. */
    async matches(executable: string): Promise<boolean> {
        return (
            (await this.#environment.files.readTextFile(this.path)) === this.describe(executable)
        );
    }

    /** Describe the foreground machine as a unit. */
    describe(executable: string): string {
        const program = JSON.stringify(executable.replaceAll("%", "%%"));
        const environment = JSON.stringify(
            `DESTACK_DIRECTORY=${this.directory}`.replaceAll("%", "%%"),
        );

        return `[Unit]\nDescription=Destack machine\n\n[Service]\nType=exec\nExecStart=:${program}\nEnvironment=${environment}\nRestart=on-failure\nRestartSec=2\nTimeoutStopSec=20\n\n[Install]\nWantedBy=default.target\n`;
    }
}
