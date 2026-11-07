import { isAbsolute, join, relative, resolve } from "node:path";
import type { CliEnvironment } from "../context/context.ts";
import { Setup } from "./setup.ts";
import { Release } from "@destack/update/release";

/** The stable launcher, forwarding to the current installed distribution. */
export const Launcher = {
    /** Forward the stable CLI launcher to the current installed distribution. */
    async forward(
        words: readonly string[],
        environment: CliEnvironment,
    ): Promise<number | undefined> {
        // run here on a Mac and from sources, which have no launcher
        const { os } = environment.host;
        if (os === "darwin" || !environment.process.runtime.isStandalone) {
            return undefined;
        }

        // read where this executable runs from and the current release
        const installer = (await Setup.current(environment)).installer();
        const inside = relative(installer.directory, resolve(environment.process.executable));
        const isRelease = !inside.startsWith("..") && !isAbsolute(inside);
        const [group, verb] = words;
        const isInstaller =
            group === "self" &&
            (verb === "install" || (verb === "uninstall" && words.includes("--native")));
        const current = await installer.current();

        // run here as a release's executable, as the downloaded installer, and before any install
        if (isRelease || isInstaller || current === undefined) {
            return undefined;
        }

        // run the current release's command, passing the shell's input, output and status through
        const command = environment.process.spawn(
            [join(current.directory, "bin", Release.executable("destack", os)), ...words],
            { stdin: "inherit", stdout: "inherit", stderr: "inherit" },
        );

        return await command.exited;
    },
};
