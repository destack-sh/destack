import { basename, dirname, join } from "node:path";
import type { Environment } from "@destack/environment";
import { DesktopLock } from "./lock.ts";
import { Distribution } from "@destack/update";
import { DesktopError } from "../error/index.ts";

/** The environment a launch through Launch Services passes on. */
const LAUNCH_ENVIRONMENT = ["DESTACK_DIRECTORY", "DESTACK_UPDATE_URL"];

/** The desktop application of a home, as the terminal and installers launch it. */
export class Application {
    /** The directory with the machine's state. */
    readonly home: string;
    /** The environment the desktop runs in. */
    readonly environment: Environment<"files" | "process" | "host">;
    /** The lock a running desktop of the home has. */
    readonly #lock: DesktopLock;

    /** Find the desktop application of a home in an environment. */
    constructor(home: string, environment: Environment<"files" | "process" | "host">) {
        this.home = home;
        this.environment = environment;
        this.#lock = new DesktopLock(home, environment);
    }

    /** Report whether a desktop runs. */
    async isRunning(): Promise<boolean> {
        return this.#lock.isHeld();
    }

    /** Wait until the running desktop quits, as it does once its machine stops. */
    async untilQuit(): Promise<void> {
        await this.#lock.released();
    }

    /** Locate the desktop application in a release's directory. */
    locateIn(release: string): string {
        const isMac = this.environment.host.os === "darwin";

        return join(release, isMac ? "Destack.app" : "Destack");
    }

    /** Locate the desktop application this executable belongs to. */
    locate(): string {
        // read this executable's directory and the operating system
        const { executable } = this.environment.process;
        const directory = dirname(executable);

        // take the Mac application containing this executable
        if (this.environment.host.os === "darwin") {
            const application = Distribution.bundle(executable);
            if (application === undefined) {
                throw DesktopError.of(
                    "APPLICATION_BLOCKED",
                    "cannot run the desktop: this command must run inside the Destack application",
                );
            }

            return application;
        }
        // take the application whose helpers directory has this executable
        else if (basename(directory) === "helpers") {
            return dirname(directory);
        }
        // take the application beside the public bin directory
        else {
            return join(dirname(directory), "Destack");
        }
    }

    /** Launch the desktop application with a link, or with none to start at its windows. */
    async launch(application: string, link: string | null): Promise<void> {
        // require the application
        const { files, process, host } = this.environment;
        await files.stat(application);

        // let Launch Services start the application on macOS, passing the link it routes to a running one
        if (host.os === "darwin") {
            const environment = LAUNCH_ENVIRONMENT.flatMap((name) => {
                const value = process.variables[name];

                return value === undefined ? [] : ["--env", `${name}=${value}`];
            });
            const child = process.spawn(
                [
                    "/usr/bin/open",
                    "-a",
                    application,
                    ...environment,
                    ...(link === null ? [] : [link]),
                ],
                { stdin: "ignore", stdout: "ignore", stderr: "inherit" },
            );
            const code = await child.exited;
            if (code !== 0) {
                throw DesktopError.of(
                    "APPLICATION_LAUNCH_BROKEN",
                    `the desktop did not launch: '${code}'`,
                );
            }
        }
        // start the executable elsewhere, detached and so without an exit to wait for, once it exists
        else {
            const executable = join(application, host.os === "win32" ? "Destack.exe" : "Destack");
            await files.stat(executable);
            process.spawn([executable, ...(link === null ? [] : [link])], {
                isDetached: true,
                stdin: "ignore",
                stdout: "ignore",
                stderr: "inherit",
            });
        }
    }
}
