import { Tarball, TarballEntry } from "@destack/package/archive";
import { type Compression, Pipe } from "@destack/environment";
import type { BuildReader } from "@destack/package/manifest";
import { schema, Text } from "@destack/schema";
import { RegistryError } from "../error/index.ts";

/** The directory npm clients expect every file of a tarball below. */
const ROOT = "package";

/** The file npm clients read a release's name, version and dependencies from. */
const PACKAGE_FILE = "package.json";

/** The fields of a package file listing dependencies by range. */
const DEPENDENCY_FIELDS = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
] as const;

/** The start of a range selecting a package of the same workspace, as Bun and pnpm write it. */
const WORKSPACE_RANGE = "workspace:";

/** A release as npm clients install it: its build's files below package/, at released versions. */
export const NpmTarball = {
    /** List a build's files as entries in path order, its package file at the linked versions. */
    async entries(
        reader: BuildReader,
        versions: Readonly<Record<string, string>>,
    ): Promise<TarballEntry[]> {
        // take the files in path order, so equal builds give equal bytes
        const files = (await reader.files()).toSorted((left, right) =>
            Text.compare(left.path, right.path),
        );

        return Promise.all(
            files.map(async (file) => {
                // release the package file, and take every other file as the build has it
                const bytes = await reader.load(file.path);
                const contents =
                    file.path === PACKAGE_FILE ? NpmTarball.release(bytes, versions) : bytes;

                return TarballEntry.of(`${ROOT}/${file.path}`, contents);
            }),
        );
    },

    /** Write a package file with each workspace range replaced by the linked release's version. */
    release(
        authored: Uint8Array,
        versions: Readonly<Record<string, string>>,
    ): Uint8Array<ArrayBuffer> {
        // replace the workspace ranges of each dependency field it has
        const manifest = schema.jsonObject().parse(JSON.parse(new TextDecoder().decode(authored)));
        const released: Record<string, unknown> = { ...manifest };
        for (const field of DEPENDENCY_FIELDS) {
            const ranges = schema
                .record(schema.string(), schema.string())
                .safeParse(manifest[field]);
            if (ranges.success) {
                released[field] = Object.fromEntries(
                    Object.entries(ranges.data).map(([name, range]) => [
                        name,
                        range.startsWith(WORKSPACE_RANGE)
                            ? NpmTarball.linked(name, versions)
                            : range,
                    ]),
                );
            }
        }

        return new TextEncoder().encode(`${JSON.stringify(released, null, 4)}\n`);
    },

    /** Read the version a workspace dependency's release has, refusing one the lock lacks. */
    linked(name: string, versions: Readonly<Record<string, string>>): string {
        const version = versions[name];
        if (version === undefined) {
            throw RegistryError.of(
                "MANIFEST_INVALID",
                `dependency '${name}' is invalid: its release is not linked`,
            );
        }

        return version;
    },

    /** Stream the gzipped archive of some entries. */
    stream(entries: readonly TarballEntry[], compression: Compression): ReadableStream<Uint8Array> {
        const archived = Tarball.stream(
            (async function* () {
                yield* entries;
            })(),
        );

        return Pipe.through(archived, compression.compress("gzip"));
    },
};
