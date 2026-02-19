import packageJson from "../package.json";

/** The backend marker for this package. */
export const BACKEND = "wasm" as const;

/** Return the package version. */
export function version(): string {
    return packageJson.version;
}
