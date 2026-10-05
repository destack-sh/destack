import { unplugin } from "@stylexjs/unplugin";
import type { BuildExtension, Plugin } from "@destack/package/build";

/** Compile StyleX styles in builds of packages that depend on `@destack/style`. */
export const styleExtension: BuildExtension = {
    transform: ({ directory }) => {
        // read the plugin StyleX declares without a type
        const plugin: unknown = unplugin.vite({
            importSources: ["@stylexjs/stylex", "@destack/style"],
            useCSSLayers: true,
            unstable_moduleResolution: { rootDir: directory, type: "commonJS" },
            cssInjectionTarget: (path) => path.includes("entry-client"),
        });
        if (!isPlugin(plugin)) {
            throw new TypeError("stylex returned no vite plugin");
        }

        return [plugin];
    },
};

/** Check that a value is a Vite plugin, which always has a name. */
function isPlugin(value: unknown): value is Plugin {
    return (
        typeof value === "object" &&
        value !== null &&
        "name" in value &&
        typeof value.name === "string"
    );
}
