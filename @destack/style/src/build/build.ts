import { unplugin, type UserOptions } from "@stylexjs/unplugin";
import type { BuildExtension, Plugin } from "@destack/package/build";

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension: BuildExtension = {
    transform: ({ directory }) => [
        styleX({
            ...styleOptions(directory),
            cssInjectionTarget: (path) => path.includes("entry-client"),
        }),
    ],
};

/** The StyleX options of a package's styles, resolving modules from its directory. */
export function styleOptions(directory: string): Partial<UserOptions> {
    return {
        importSources: ["@stylexjs/stylex", "@destack/style"],
        useCSSLayers: true,
        unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
    };
}

/** Compile StyleX styles in Vite. */
export function styleX(options: Partial<UserOptions>): Plugin {
    // read the plugin StyleX declares without a type
    const plugin: unknown = unplugin.vite(options);
    if (!isPlugin(plugin)) {
        throw new TypeError("stylex returned no vite plugin");
    }

    return plugin;
}

/** Check that a value is a Vite plugin, which always has a name. */
function isPlugin(value: unknown): value is Plugin {
    return (
        typeof value === "object" &&
        value !== null &&
        "name" in value &&
        typeof value.name === "string"
    );
}
