import type { BuildStore } from "@destack/build/store";
import { type Environment, Pipe } from "@destack/environment";
import { PackageJson } from "@destack/package";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { type BuildReader, BuildImage } from "@destack/package/manifest";
import { type Digest, present, schema, Text } from "@destack/schema";
import { NpmRelease } from "./packument.ts";

/** The directory npm clients expect every file of a tarball below. */
const ROOT = "package";

/** The file npm clients read a release's name, version and dependencies from. */
const PACKAGE_FILE = "package.json";

/** A release as npm clients install it: its sources below package/, at released versions. */
export const NpmTarball = {
    /** Archive an image's root build at the versions the image links, with what a listing says. */
    async pack(
        store: Pick<BuildStore, "image" | "get" | "contents">,
        image: Digest,
        environment: Pick<Environment, "crypto" | "compression">,
    ): Promise<{ readonly bytes: Uint8Array<ArrayBuffer>; readonly listed: NpmRelease }> {
        // read the version each of the root's requirements links
        const opened = await store.image(image);
        const links = Object.entries(BuildImage.links(opened, opened.root));
        const linked = await Promise.all(
            links.map(async ([name, build]) => {
                const { version } = (await store.get(build)).package;

                return [name, version] as const;
            }),
        );

        return NpmTarball.of(
            await store.contents(opened.root),
            Object.fromEntries(linked),
            environment,
        );
    },

    /** Archive a build's sources at the versions its requirements link, with its listing. */
    async of(
        reader: BuildReader,
        versions: Readonly<Record<string, string>>,
        environment: Pick<Environment, "crypto" | "compression">,
    ): Promise<{ readonly bytes: Uint8Array<ArrayBuffer>; readonly listed: NpmRelease }> {
        // take the files a developer's node_modules reads, leaving what each runtime runs
        const compiled = Object.values(reader.manifest.outputs).map(
            (output) => `${output.directory}/`,
        );
        const files = (await reader.files())
            .filter((file) => !compiled.some((directory) => file.path.startsWith(directory)))
            .toSorted((left, right) => Text.compare(left.path, right.path));
        const loaded = await Promise.all(
            files.map(async (file) => [file.path, await reader.load(file.path)] as const),
        );

        // release the package file, and archive every other file as the build has it
        const authored = present(
            loaded.find(([path]) => path === PACKAGE_FILE),
            "the build's package file",
        )[1];
        const manifest = PackageJson.release(
            schema.jsonObject().parse(JSON.parse(new TextDecoder().decode(authored))),
            versions,
        );
        const released = new TextEncoder().encode(`${JSON.stringify(manifest, null, 4)}\n`);
        const entries = loaded.map(([path, bytes]) =>
            TarballEntry.of(`${ROOT}/${path}`, path === PACKAGE_FILE ? released : bytes),
        );

        // compress the archive, and hash it as npm's subresource integrity
        const archived = Tarball.stream(
            (async function* () {
                yield* entries;
            })(),
        );
        const zipped = Pipe.through(archived, environment.compression.compress("gzip"));
        const bytes = new Uint8Array(await new Response(zipped).arrayBuffer());
        const digest = new Uint8Array(await environment.crypto.digest("SHA-512", bytes));

        return {
            bytes,
            listed: {
                manifest: NpmRelease.installed(manifest),
                integrity: `sha512-${digest.toBase64()}`,
            },
        };
    },
};
