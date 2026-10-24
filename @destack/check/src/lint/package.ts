import { schema } from "@destack/schema";

/** A tool's configuration module, such as `vitest.config.ts`, which the tool runs as a program. */
const CONFIGURATION = /\/[\w-]+\.config\.[cm]?[jt]s$/u;

/** The package whose host modules define the system effects. */
const ENVIRONMENT_PACKAGE = /[\\/]@destack[\\/]environment[\\/]/u;

/** The Destack settings of a lint configuration, as configure writes them. */
const lintSettings = schema.object({
    /** The package directories relative to the settings root, "." for the root. */
    packages: schema.array(schema.string()),
    /** The modules the packages start: `bin`, entrypoints, Bun scripts and preloads. */
    entrypoints: schema.array(schema.string()),
});

/** The Destack settings of a lint configuration, as configure writes them. */
export type LintSettings = schema.Infer<typeof lintSettings>;

/** The Destack settings of a lint configuration, as configure writes them. */
export const LintSettings = Object.assign(lintSettings, {
    /** Read the settings a rule receives, refusing a configuration without them. */
    read(settings: unknown): LintSettings {
        const parsed = schema.looseObject({ destack: lintSettings }).safeParse(settings);
        if (!parsed.success) {
            throw new TypeError("the lint configuration lists no Destack packages; run configure");
        }

        return parsed.data.destack;
    },
});

/** Report whether a source file belongs to a Destack package the lint settings list. */
export function isPackageFile(filename: string, settings: unknown): boolean {
    const path = filename.replaceAll("\\", "/");

    return LintSettings.read(settings).packages.some(
        (directory) => directory === "." || path.includes(`/${directory}/`),
    );
}

/** Report whether a source file is a program entrypoint or a tool's configuration module. */
export function isEntrypoint(filename: string, settings: unknown): boolean {
    const path = filename.replaceAll("\\", "/");

    return (
        CONFIGURATION.test(path) ||
        LintSettings.read(settings).entrypoints.some((entrypoint) =>
            path.endsWith(`/${entrypoint}`),
        )
    );
}

/** Report whether a source file belongs to the environment package. */
export function isEnvironmentFile(filename: string): boolean {
    return ENVIRONMENT_PACKAGE.test(filename);
}
