import type { Environment } from "@destack/environment";
import { dirname, join, resolve } from "node:path";
import { Distribution, Release } from "@destack/update";
import type { DistributionHost } from "@destack/update/server";
import type {} from "@destack/package/import-meta";
import { Stage } from "@destack/update/release";

/** A stage installed at a device directory for one user. */
export class StageHome {
    /** The stage installed here. */
    readonly stage: Stage;
    /** The device directory, the stage's own in the user's home unless one is set. */
    readonly directory: string;
    /** The origin of the universe people sign in at, the stage's unless one is set. */
    readonly universe: string;
    /** The user's home directory, with the stage's own directory and applications. */
    readonly #user: string;

    /** Place a stage in a user's home, in its own device directory at its universe unless given. */
    constructor(
        stage: Stage,
        user: string,
        directory: string = join(user, stage.directory),
        universe: string = stage.universe,
    ) {
        // take the stage, the device directory, the universe and the user's home
        this.stage = stage;
        this.directory = directory;
        this.universe = universe;
        this.#user = user;
    }

    /** Read the home of the stage the build selected, with its directory and universe set. */
    static current(environment: Environment<"process" | "host">): StageHome {
        const stage = Stage.current(environment);
        const { variables } = environment.process;

        return new StageHome(
            stage,
            environment.host.home,
            variables["DESTACK_DIRECTORY"],
            variables["DESTACK_UNIVERSE"],
        );
    }

    /** Whether the device directory is an explicit one, apart from the stage's own. */
    get isCustom(): boolean {
        return resolve(this.directory) !== join(this.#user, this.stage.directory);
    }

    /** The installed distribution of this stage's executables. */
    distribution(environment: Environment<"files" | "process" | "host">): DistributionHost {
        // describe the installation this executable belongs to
        const { stage } = this;
        const { variables, executable } = environment.process;
        const { os, architecture } = environment.host;
        const applications = this.isCustom ? resolve(this.directory) : this.#user;
        const distribution = new Distribution({
            home: this.directory,
            repository: new URL(variables["DESTACK_UPDATE_URL"] ?? stage.feed),
            root: Distribution.root(),
            running: new Release(
                import.meta.destack.package.version,
                Release.target(os, architecture),
            ),
            executable,
            applications: join(applications, "Applications"),
            stage,
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
