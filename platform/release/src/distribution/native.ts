import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import type { Target } from "@destack/update/release";

/** Published Turso native packages selected by distribution target. */
const TURSO: Partial<Record<Target, string>> = {
    "aarch64-apple-darwin": "@tursodatabase/database-darwin-arm64",
    "aarch64-unknown-linux-gnu": "@tursodatabase/database-linux-arm64-gnu",
    "x86_64-unknown-linux-gnu": "@tursodatabase/database-linux-x64-gnu",
    "x86_64-pc-windows-msvc": "@tursodatabase/database-win32-x64-msvc",
};

/** Embed the selected native addon through Bun's static require expression. */
export function nativePlugin(target: Target): Bun.BunPlugin {
    return {
        name: "destack-native-addon",
        setup(build) {
            // replace Turso's runtime platform lookup with the known distribution target
            build.onLoad(
                { filter: /[/\\]@tursodatabase[/\\]database[/\\]index\.js$/ },
                (arguments_) => {
                    // require a native addon for the exact distribution target
                    const name = TURSO[target];
                    if (!name && target !== "x86_64-apple-darwin") {
                        throw new Error(`no Turso native addon is published for ${target}`);
                    }
                    const require = createRequire(arguments_.path);
                    const native = name
                        ? require.resolve(name)
                        : require.resolve(
                              fileURLToPath(
                                  new URL(
                                      "../../../../dist/native/turso-x86_64-apple-darwin.node",
                                      import.meta.url,
                                  ),
                              ),
                          );

                    return {
                        loader: "js",
                        contents: `import native from ${JSON.stringify(native)}; export const { Database, EncryptionCipher } = native;`,
                    };
                },
            );
        },
    };
}
