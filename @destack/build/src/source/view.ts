import { relative } from "node:path";
import { PackageError } from "@destack/package/error";
import type { ApplicationOptions } from "../compile/application.ts";
import { readPackageDeclaration } from "./source.ts";

/** Resolve a named view through the package's browser exports. */
export async function resolveView(
    directory: string,
    application: ApplicationOptions,
): Promise<ApplicationOptions> {
    if (application.view === undefined) {
        return application;
    }
    if (application.app !== undefined) {
        throw new PackageError("INVALID_DEFINITION", "select a view or an app source path");
    }

    // apply the same conditional export selection used by compilation
    const source = await readPackageDeclaration(directory, "browser");
    const view = source.declaration.definition.views?.[application.view];
    const entrypoint = view && source.exports[view.entrypoint];
    if (!entrypoint) {
        throw new PackageError("INVALID_DEFINITION", `unknown browser view: ${application.view}`);
    }

    return { ...application, app: relative(source.directory, entrypoint).replaceAll("\\", "/") };
}
