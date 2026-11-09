import { resolve } from "node:path";
import type { CheckOptions } from "./check.ts";

/** The folders a package keeps its code and tests in, checked when a call selects none. */
const PACKAGE_SOURCES = ["src", "tests"] as const;

/** Select a call's files, else the package's source and test folders it has. */
export async function selectFiles(
    options: Pick<CheckOptions, "directory" | "files" | "environment">,
): Promise<string[]> {
    // take the call's selection, else each package folder that exists
    if (options.files !== undefined) {
        return options.files;
    }
    const present = await Promise.all(
        PACKAGE_SOURCES.map(async (folder) =>
            (await options.environment.files.exists(resolve(options.directory, folder)))
                ? [folder]
                : [],
        ),
    );

    return present.flat();
}
