import type { Environment } from "@destack/environment";
import { dirname, join, resolve } from "node:path";
import { Distribution, Release } from "@destack/update";
import type { DistributionHost } from "@destack/update/server";
import type {} from "@destack/package/import-meta";
import { Universe } from "@destack/update/release";

/** A universe installed at a device directory for one user. */
export class UniverseHome {
    /** The universe installed here, with the origin people sign in at. */
    readonly universe: Universe;
    /** The device directory, the universe's own in the user's home unless one is set. */
    readonly directory: string;
    /** The user's home directory, with the universe's own directory and applications. */
    readonly #user: string;

    /** Place a universe in a user's home, in its own device directory unless one is given. */
    constructor(
        universe: Universe,
        user: string,
        directory: string = join(user, universe.directory),
    ) {
        // take the universe, the device directory and the user's home
        this.universe = universe;
        this.directory = directory;
        this.#user = user;
    }

    /** Read the home of the universe the build selected, with its directory set. */
    static current(environment: Environment<"process" | "host">): UniverseHome {
        return new UniverseHome(
            Universe.current(environment),
            environment.host.home,
            environment.process.variables["DESTACK_DIRECTORY"],
        );
    }

    /** Whether the device directory is an explicit one, apart from the universe's own. */
    get isCustom(): boolean {
        return resolve(this.directory) !== join(this.#user, this.universe.directory);
    }

    /** The installed distribution of this universe's executables. */
    distribution(environment: Environment<"files" | "process" | "host">): DistributionHost {
        // describe the installation this executable belongs to
        const { universe } = this;
        const { variables, executable } = environment.process;
        const { os, architecture } = environment.host;
        const applications = this.isCustom ? resolve(this.directory) : this.#user;
        const distribution = new Distribution({
            home: this.directory,
            repository: new URL(variables["DESTACK_UPDATE_URL"] ?? universe.feed),
            root: Distribution.root(),
            running: new Release(
                import.meta.destack.package.version,
                Release.target(os, architecture),
            ),
            executable,
            applications: join(applications, "Applications"),
            universe,
            environment,
        });

        // activate through the installed command, detached as it replaces this process
        const command = join(dirname(executable), os === "win32" ? "destack.exe" : "destack");
        const activate = () => {
            environment.process.spawn([command, "self", "activate"], {
                isDetached: true,
                stdin: "ignore",
                stdout: "ignore",
                stderr: "ignore",
            });
        };

        return { distribution, activate };
    }
}
