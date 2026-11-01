import { join } from "node:path";
import type { AutostartRegistration } from "./autostart.ts";
import { Command, type CommandHost, SERVICE_MILLISECONDS } from "../process/command.ts";

/** The permissions of the property list: the person's alone. */
const REGISTRATION_MODE = 0o600;

/** The exit status `launchctl print` reports for a job it cannot find. */
const LAUNCHCTL_UNLOADED = 113;

/** A launchd agent running the machine in the user's GUI domain. */
export class LaunchAgent implements AutostartRegistration {
    /** The job label. */
    readonly name: string;
    /** The machine state directory. */
    readonly directory: string;
    /** The property list in the user's LaunchAgents directory. */
    readonly path: string;
    /** Whether launchd supervises the running machine: it does. */
    readonly isSupervised = true;
    /** The host launchd runs on. */
    readonly #environment: CommandHost;

    /** Locate the agent of a job label and machine directory on a host. */
    constructor(name: string, directory: string, environment: CommandHost) {
        // set the label and directory, the property list in the person's LaunchAgents, and the host
        this.name = name;
        this.directory = directory;
        this.path = join(environment.host.home, "Library/LaunchAgents", `${name}.plist`);
        this.#environment = environment;
    }

    /** Publish the complete property list atomically through a temporary file. */
    async install(executable: string, temporary: string): Promise<void> {
        const { files } = this.#environment;
        await files.writeTextFile(temporary, this.describe(executable), {
            createNew: true,
            mode: REGISTRATION_MODE,
        });
        await files.rename(temporary, this.path);
    }

    /** Load an unloaded job, or restart its stopped foreground process. */
    async start(): Promise<void> {
        // read whether the job is loaded in the person's GUI domain
        const domain = `gui/${await this.#userIdentifier()}`;
        const { code } = await Command.run(
            this.#environment,
            ["launchctl", "print", `${domain}/${this.name}`],
            { timeout: SERVICE_MILLISECONDS, accepted: [LAUNCHCTL_UNLOADED] },
        );
        // load an unloaded job
        if (code === LAUNCHCTL_UNLOADED) {
            await Command.run(this.#environment, ["launchctl", "bootstrap", domain, this.path], {
                timeout: SERVICE_MILLISECONDS,
            });
        }
        // restart a loaded job
        else {
            await Command.run(
                this.#environment,
                ["launchctl", "kickstart", `${domain}/${this.name}`],
                { timeout: SERVICE_MILLISECONDS },
            );
        }
    }

    /** Unload the job so launchd cannot restart it during an update. */
    async stop(): Promise<void> {
        // unload the job when it is loaded
        const job = `gui/${await this.#userIdentifier()}/${this.name}`;
        const { code } = await Command.run(this.#environment, ["launchctl", "print", job], {
            timeout: SERVICE_MILLISECONDS,
            accepted: [LAUNCHCTL_UNLOADED],
        });
        if (code === 0) {
            await Command.run(this.#environment, ["launchctl", "bootout", job], {
                timeout: SERVICE_MILLISECONDS,
            });
        }
    }

    /** Remove the property list of the stopped job. */
    async uninstall(): Promise<void> {
        await this.#environment.files.remove(this.path);
    }

    /** Report whether the registered property list runs an executable. */
    async matches(executable: string): Promise<boolean> {
        return (
            (await this.#environment.files.readTextFile(this.path)) === this.describe(executable)
        );
    }

    /** Describe the foreground machine as a property list, escaping its strings. */
    describe(executable: string): string {
        // escape the program, the state directory and the log
        const program = LaunchAgent.#xml(executable);
        const directory = LaunchAgent.#xml(this.directory);
        const log = LaunchAgent.#xml(join(this.directory, "machine.log"));

        return `<?xml version="1.0" encoding="UTF-8"?>\n<plist version="1.0"><dict>
<key>Label</key><string>${this.name}</string>
<key>ProgramArguments</key><array><string>${program}</string></array>
<key>EnvironmentVariables</key><dict><key>DESTACK_DIRECTORY</key><string>${directory}</string></dict>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
<key>ExitTimeOut</key><integer>20</integer>
<key>StandardOutPath</key><string>${log}</string>
<key>StandardErrorPath</key><string>${log}</string>
</dict></plist>\n`;
    }

    /** Read the user identifier of launchd's GUI domain, as `id -u` prints it. */
    async #userIdentifier(): Promise<string> {
        const { output } = await Command.run(this.#environment, ["id", "-u"], {
            timeout: SERVICE_MILLISECONDS,
        });

        return output.trim();
    }

    /** Escape text embedded in an XML property list. */
    static #xml(value: string): string {
        return value
            .replaceAll("&", "&amp;")
            .replaceAll("<", "&lt;")
            .replaceAll(">", "&gt;")
            .replaceAll('"', "&quot;")
            .replaceAll("'", "&apos;");
    }
}
