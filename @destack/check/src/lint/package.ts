import { PackageRole } from "@destack/package";
import { schema } from "@destack/schema";

/** A tool's configuration module, such as `vitest.config.ts`, which the tool runs as a program. */
const CONFIGURATION = /\/[\w-]+\.config\.[cm]?[jt]s$/u;

/** The package whose host modules define the system effects. */
const ENVIRONMENT_PACKAGE = /[\\/]@destack[\\/]environment[\\/]/u;

/** The Destack settings of a lint configuration, as configure writes them. */
const lintSettings = schema.object({
    /** The packages, by directory relative to the settings root ("." for the root), name and description. */
    packages: schema.array(
        schema.object({
            directory: schema.string(),
            name: schema.string(),
            description: schema.string().exactOptional(),
        }),
    ),
    /** The role of each Destack package the packages are or depend on, by name. */
    roles: schema.record(schema.string(), PackageRole),
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
    return packageOf(filename, settings) !== undefined;
}

/** Find the package of a source file among those the lint settings list, the innermost one. */
export function packageOf(
    filename: string,
    settings: unknown,
): LintSettings["packages"][number] | undefined {
    const path = filename.replaceAll("\\", "/");
    const containing = LintSettings.read(settings).packages.filter(
        ({ directory }) => directory === "." || path.includes(`/${directory}/`),
    );

    return containing.toSorted((left, right) => right.directory.length - left.directory.length)[0];
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
