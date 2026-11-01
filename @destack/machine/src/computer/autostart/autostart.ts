import { Uuid } from "@destack/schema";
import { dirname, resolve } from "node:path";
import type { CommandHost } from "../process/command.ts";
import { LaunchAgent } from "./launchd.ts";
import { SystemdService } from "./systemd.ts";
import { StartupShortcut } from "./startup.ts";

/** The hexadecimal digits of the directory's digest in a registration's name. */
const NAME_DIGEST_LENGTH = 16;

/** One operating system's per-user registration of the machine. */
export interface AutostartRegistration {
    /** The registration file the operating system reads. */
    readonly path: string;
    /** Write the registration of an executable through a temporary path beside it. */
    readonly install: (executable: string, temporary: string) => Promise<void>;
    /** Start the registered process where the system supervises it. */
    readonly start: () => Promise<void>;
    /** Stop the system's supervision of the registered process. */
    readonly stop: () => Promise<void>;
    /** Remove the registration of the stopped process. */
    readonly uninstall: () => Promise<void>;
    /** Whether the system supervises the registered process, starting and stopping it. */
    readonly isSupervised: boolean;
    /** Report whether the registration runs an executable. */
    readonly matches: (executable: string) => Promise<boolean>;
}

/** Per-user OS registration for one machine directory. */
export class Autostart {
    /** Absolute machine state directory. */
    readonly directory: string;
    /** Stable OS job identifier, independent of application version. */
    readonly name: string;
    /** Registration file read by the user's operating system. */
    readonly path: string;
    /** The current operating system's registration. */
    readonly #registration: AutostartRegistration;
    /** The host the registration lives on. */
    readonly #environment: CommandHost;

    /** Take the OS registration of a machine directory under its job name on a host. */
    constructor(directory: string, name: string, environment: CommandHost) {
        // set the directory, the job name and the host
        this.directory = directory;
        this.name = name;
        this.#environment = environment;

        // use the current user's standard startup registration
        const { os } = environment.host;
        // use launchd on macOS
        if (os === "darwin") {
            this.#registration = new LaunchAgent(this.name, this.directory, environment);
        }
        // use systemd on Linux
        else if (os === "linux") {
            this.#registration = new SystemdService(this.name, this.directory, environment);
        }
        // use a startup shortcut on Windows
        else {
            this.#registration = new StartupShortcut(this.name, environment);
        }
        this.path = this.#registration.path;
    }

    /** Locate the OS registration of a machine directory on a host, named by its digest. */
    static async of(directory: string, environment: CommandHost): Promise<Autostart> {
        // separate the registrations of independent directories by a stable digest of the directory
        const resolved = resolve(directory);
        const bytes = new TextEncoder().encode(resolved);
        const digest = new Uint8Array(await environment.crypto.digest("SHA-256", bytes)).toHex();

        return new Autostart(
            resolved,
            `app.destack.machine.${digest.slice(0, NAME_DIGEST_LENGTH)}`,
            environment,
        );
    }

    /** Read whether this machine has an OS registration. */
    async isRegistered(): Promise<boolean> {
        return this.#environment.files.exists(this.path);
    }

    /** Whether the system supervises the registered machine, starting and stopping it. */
    get isSupervised(): boolean {
        return this.#registration.isSupervised;
    }

    /** Report whether the machine is registered to run an executable. */
    async matches(executable: string): Promise<boolean> {
        return (await this.isRegistered()) && this.#registration.matches(executable);
    }

    /** Register the selected machine executable, or the Windows CLI startup command. */
    async register(executable: string): Promise<void> {
        // stop OS supervision before replacing its registration
        const { files, random } = this.#environment;
        if (await this.isRegistered()) {
            await this.stop();
        }
        await files.mkdir(dirname(this.path), { recursive: true });

        // publish the registration through a temporary path, removing it once moved or failed
        const temporary = `${this.path}.${Uuid.v4(random)}`;
        try {
            await this.#registration.install(executable, temporary);
        } finally {
            await files.remove(temporary, { force: true });
        }
    }

    /** Start the registered foreground process on systems with user service managers. */
    async start(): Promise<void> {
        await this.#registration.start();
    }

    /** Stop OS supervision before machine shutdown or executable replacement. */
    async stop(): Promise<void> {
        await this.#registration.stop();
    }

    /** Remove this startup registration without touching machine state. */
    async unregister(): Promise<void> {
        if (!(await this.isRegistered())) {
            return;
        }
        await this.stop();
        await this.#registration.uninstall();
    }
}
