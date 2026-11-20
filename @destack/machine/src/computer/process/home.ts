import type { Environment } from "@destack/environment";
import { dirname, join, resolve } from "node:path";
import { Distribution, Release } from "@destack/update";
import type { DistributionHost } from "@destack/update/server";
import type {} from "@destack/package/import-meta";
import { Profile } from "@destack/update/release";

/** A profile installed at a device directory for one user, signing in at a universe. */
export class UniverseHome {
    /** The profile installed here, which names its commands, directory and application. */
    readonly profile: Profile;
    /** The origin of the universe people sign in at. */
    readonly universe: string;
    /** The device directory, the profile's own in the user's home unless one is set. */
    readonly directory: string;
    /** The user's home directory, with the profile's own directory and applications. */
    readonly #user: string;

    /** Place a profile in a user's home, in its own device directory unless one is given. */
    constructor(
        profile: Profile,
        universe: string,
        user: string,
        directory: string = join(user, profile.directory),
    ) {
        // take the profile, the universe, the device directory and the user's home
        this.profile = profile;
        this.universe = universe;
        this.directory = directory;
        this.#user = user;
    }

    /** Read the home of the profile the build selected, at its universe and directory when set. */
    static current(environment: Environment<"process" | "host">): UniverseHome {
        const { variables } = environment.process;
        const profile = Profile.current(environment);

        return new UniverseHome(
            profile,
            variables["DESTACK_UNIVERSE_ORIGIN"] ?? profile.universe,
            environment.host.home,
            variables["DESTACK_DIRECTORY"],
        );
    }

    /** Whether the device directory is an explicit one, apart from the profile's own. */
    get isCustom(): boolean {
        return resolve(this.directory) !== join(this.#user, this.profile.directory);
    }

    /** The installed distribution of this profile's executables. */
    distribution(environment: Environment<"files" | "process" | "host">): DistributionHost {
        // describe the installation this executable belongs to
        const { profile } = this;
        const { variables, executable } = environment.process;
        const { os, architecture } = environment.host;
        const applications = this.isCustom ? resolve(this.directory) : this.#user;
        const distribution = new Distribution({
            home: this.directory,
            repository: new URL(variables["DESTACK_UPDATE_URL"] ?? Distribution.feed()),
            root: Distribution.root(),
            running: new Release(
                import.meta.destack.package.version,
                Release.target(os, architecture),
            ),
            executable,
            applications: join(applications, "Applications"),
            profile,
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
