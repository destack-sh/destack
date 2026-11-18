import { join } from "node:path";
import { readSourceLayout } from "./sources.ts";
import type { CheckOptions } from "./check.ts";

/** The manifests of a package beside its sources: its package file and its Destack definition. */
const MANIFESTS = ["package.json", "destack.json"];

/** Select a call's files, else the package's source folders, root modules and manifests. */
export async function selectFiles(
    options: Pick<CheckOptions, "directory" | "files" | "environment">,
): Promise<string[]> {
    // take the call's selection, else every top-level source the package has
    if (options.files !== undefined) {
        return options.files;
    }
    const roots = await readSourceLayout(options.directory, options.environment.files);

    // add the manifests the package has, which the formatter formats as it formats the sources
    const { files } = options.environment;
    const manifests = [];
    for (const manifest of MANIFESTS) {
        if (await files.exists(join(options.directory, manifest))) {
            manifests.push(manifest);
        }
    }

    return [...roots.folders, ...roots.modules, ...manifests];
}
