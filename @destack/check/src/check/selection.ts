import { readSourceRoots } from "./sources.ts";
import type { CheckOptions } from "./check.ts";

/** Select a call's files, else the package's source folders and root modules. */
export async function selectFiles(
    options: Pick<CheckOptions, "directory" | "files" | "environment">,
): Promise<string[]> {
    // take the call's selection, else every top-level source the package has
    if (options.files !== undefined) {
        return options.files;
    }
    const roots = await readSourceRoots(options.directory, options.environment.files);

    return [...roots.folders, ...roots.modules];
}
