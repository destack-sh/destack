import { MachineError } from "../../error/index.ts";
import { join } from "node:path";
import type { AutostartRegistration } from "./autostart.ts";
import { Command, type CommandHost, SERVICE_MILLISECONDS } from "../process/command.ts";

/** A shortcut in the user's Startup folder running `machine start` at sign-in. */
export class StartupShortcut implements AutostartRegistration {
    /** The shortcut name without its extension. */
    readonly name: string;
    /** The shortcut in the user's Startup folder. */
    readonly path: string;
    /** Whether Windows supervises the running machine: it runs the shortcut once at sign-in. */
    readonly isSupervised = false;
    /** The host Windows runs on. */
    readonly #environment: CommandHost;

    /** Locate the shortcut of a name in the user's Startup folder on a host. */
    constructor(name: string, environment: CommandHost) {
        // require the user's roaming application directory
        this.#environment = environment;
        const applicationDirectory = environment.process.variables["APPDATA"];
        if (applicationDirectory === undefined || applicationDirectory === "") {
            throw MachineError.of(
                "CONFIGURATION_INVALID",
                "APPDATA is required for per-user startup",
            );
        }
        this.name = name;
        this.path = join(
            applicationDirectory,
            "Microsoft/Windows/Start Menu/Programs/Startup",
            `${name}.lnk`,
        );
    }

    /** Let Windows create the native shortcut without evaluating user paths as code. */
    async install(executable: string, temporary: string): Promise<void> {
        // create the shortcut beside the target
        const shortcut = `${temporary}.lnk`;
        try {
            await Command.run(
                this.#environment,
                [
                    "powershell.exe",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "$shell = New-Object -ComObject WScript.Shell; " +
                        "$shortcut = $shell.CreateShortcut($env:DESTACK_SHORTCUT); " +
                        "$shortcut.TargetPath = $env:DESTACK_EXECUTABLE; " +
                        "$shortcut.Arguments = 'machine start'; $shortcut.WindowStyle = 7; $shortcut.Save()",
                ],
                {
                    variables: {
                        ...this.#environment.process.variables,
                        DESTACK_SHORTCUT: shortcut,
                        DESTACK_EXECUTABLE: executable,
                    },
                    timeout: SERVICE_MILLISECONDS,
                },
            );
            await this.#environment.files.rename(shortcut, this.path);
        } finally {
            await this.#environment.files.remove(shortcut, { force: true });
        }
    }

    /** Start nothing, since Windows runs the shortcut at the next sign-in. */
    async start(): Promise<void> {}

    /** Stop nothing, since Windows supervises no startup shortcut. */
    async stop(): Promise<void> {}

    /** Remove the shortcut. */
    async uninstall(): Promise<void> {
        await this.#environment.files.remove(this.path);
    }

    /** Report no match, as the shortcut is a native file each setup writes again. */
    async matches(): Promise<boolean> {
        return false;
    }
}
