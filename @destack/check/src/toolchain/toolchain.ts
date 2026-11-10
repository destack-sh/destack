import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Host } from "@destack/environment";

/** The compilers and lint rules on disk: a checked-out toolchain, or a workspace's packages. */
export class Toolchain {
    /** The toolchain's directory, absent in a workspace, whose packages resolve from source. */
    readonly directory: string | undefined;

    /** Read the tools from a directory, or from the source tree when absent. */
    private constructor(directory: string | undefined) {
        this.directory = directory;
    }

    /** Read the tools a workspace installed. */
    static workspace(): Toolchain {
        return new Toolchain(undefined);
    }

    /** Read the tools of a toolchain directory. */
    static at(directory: string): Toolchain {
        return new Toolchain(directory);
    }

    /** Read the suffix of a platform's native addon packages. */
    static platform(host: Host): string {
        const { os, architecture } = host;

        return `${os}-${architecture}${os === "linux" ? "-gnu" : ""}`;
    }

    /** Find an installed package's directory among the toolchain's packages. */
    locate(chain: readonly string[], from: string): string {
        // find the last package among the toolchain's, else each package from the one before
        const names = this.directory === undefined ? chain : chain.slice(-1);
        let directory = this.directory ?? from;
        for (const name of names) {
            const require = createRequire(join(directory, "package.json"));
            directory = dirname(require.resolve(`${name}/package.json`));
        }

        return directory;
    }

    /** Find the module with this package's lint rules: the toolchain's bundle, or the source. */
    lint(): string {
        return this.directory === undefined
            ? fileURLToPath(new URL("../lint/index.ts", import.meta.url))
            : join(this.directory, "lint.js");
    }
}
