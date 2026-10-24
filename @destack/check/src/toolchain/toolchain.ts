import { createRequire } from "node:module";
import { dirname, join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import type { Platform, Process } from "@destack/environment";

/** The name of the toolchain directory beside a standalone executable. */
const TOOLCHAIN = "toolchain";

/** The tools on disk. */
export const Toolchain = {
    /** Find the toolchain directory of a standalone executable, absent in a workspace. */
    directory(process: Process): string | undefined {
        return process.runtime.isStandalone ? Toolchain.of(process.executable) : undefined;
    },

    /** Read the suffix of a platform's native addon packages. */
    platform(platform: Platform): string {
        const { os, architecture } = platform;

        return `${os}-${architecture}${os === "linux" ? "-gnu" : ""}`;
    },

    /** Find an executable's toolchain: in its macOS application's resources, else beside it. */
    of(executable: string): string {
        // read a macOS application's toolchain from its bundle's resources for code signing
        const segments = executable.split(sep);
        const contents = segments.lastIndexOf("Contents");
        if (segments[contents - 1]?.endsWith(".app") === true) {
            return join(segments.slice(0, contents + 1).join(sep), "Resources", TOOLCHAIN);
        }

        // read the toolchain beside any other executable
        return join(dirname(executable), TOOLCHAIN);
    },

    /** Find an installed package's directory among the toolchain's packages. */
    locate(chain: readonly string[], from: string, process: Process): string {
        // find the last package among the toolchain's, else each package from the one before
        const toolchain = Toolchain.directory(process);
        const names = toolchain === undefined ? chain : chain.slice(-1);
        let directory = toolchain ?? from;
        for (const name of names) {
            const require = createRequire(join(directory, "package.json"));
            directory = dirname(require.resolve(`${name}/package.json`));
        }

        return directory;
    },

    /** Find the module with this package's lint rules: the toolchain's bundle, or the source. */
    lint(process: Process): string {
        const toolchain = Toolchain.directory(process);

        return toolchain === undefined
            ? fileURLToPath(new URL("../lint/index.ts", import.meta.url))
            : join(toolchain, "lint.js");
    },
};
