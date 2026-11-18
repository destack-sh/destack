import type { Crypto } from "@destack/environment";
import { canonicalize, Digest, present, schema } from "@destack/schema";
import type { DependencyRelease } from "../definition/dependency.ts";
import { DependencyName } from "../definition/package.ts";
import type { PackageManifest } from "./manifest.ts";

/** The image format a store writes. */
export const IMAGE_FORMAT = 2;

/** A root build and every build it runs with, each with the build its requirements resolve to. */
const image = schema
    .object({
        /** The image format version. */
        formatVersion: schema.literal(IMAGE_FORMAT),
        /** The digest of the root build's manifest. */
        root: Digest,
        /** Each build, the root's among them, with the build each requirement resolves to. */
        builds: schema.record(Digest, schema.record(DependencyName, Digest)),
    })
    .superRefine((value, context) => {
        // require the root and every resolved build among the builds
        const targets = [
            value.root,
            ...Object.values(value.builds).flatMap((each) => Object.values(each)),
        ];
        for (const target of targets) {
            if (value.builds[target] === undefined) {
                context.addIssue({
                    code: "custom",
                    path: ["builds", target],
                    message: `build '${target}' is missing: the image resolves to it`,
                });
            }
        }
    });

/** A root build and every build it runs with, each with the build its requirements resolve to. */
export const BuildImage = Object.assign(image, {
    /** Make an image of a root build and its builds, the root linking none by default. */
    of(
        root: Digest,
        builds: Readonly<Record<Digest, Readonly<Record<string, Digest>>>> = {},
    ): BuildImage {
        return { formatVersion: IMAGE_FORMAT, root, builds: { [root]: {}, ...builds } };
    },

    /** Make an image of a root with one build per package, hoisted as npm does, linked by name. */
    async hoisted(
        root: Digest,
        builds: Readonly<Record<string, Digest>>,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<BuildImage> {
        // link each build's requirements and peers to the builds of those names
        const links: Record<Digest, Record<string, Digest>> = {};
        for (const build of [root, ...Object.values(builds)]) {
            const manifest = await read(build);
            const peers = Object.values(manifest.outputs).flatMap((output) =>
                Object.keys(output.peers),
            );
            const resolved = [...Object.keys(manifest.requires), ...peers].flatMap((name) => {
                const target = builds[name];

                return target === undefined || target === build ? [] : [[name, target] as const];
            });
            links[build] = Object.fromEntries(resolved);
        }

        return BuildImage.of(root, links);
    },

    /** Serialize an image as its canonical bytes, whose digest identifies it. */
    serialize(value: BuildImage): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(canonicalize(value));
    },

    /** Read an image's bytes from a store that verified their digest, as its writer wrote them. */
    deserialize(bytes: Uint8Array<ArrayBuffer>): BuildImage {
        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- the verified digest fixes the bytes a parsed image wrote
        return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as BuildImage;
    },

    /** Take the digest of an image, its address in every store. */
    digest(value: BuildImage, crypto: Crypto): Promise<Digest> {
        return Digest.of(BuildImage.serialize(value), crypto);
    },

    /** Read every build of an image with its package: the root's, its links, then the rest. */
    async releases(
        value: BuildImage,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<DependencyRelease[]> {
        // order the root and its own links first, so their names come before nested ones
        const linked = Object.values(BuildImage.links(value, value.root));
        const ordered = [...new Set([value.root, ...linked, ...BuildImage.builds(value)])];

        return await Promise.all(
            ordered.map(async (manifest): Promise<DependencyRelease> => {
                // take a Destack package's build, else an npm unit's
                const { package: built } = await read(manifest);

                return "id" in built
                    ? { kind: "destack", package: built, manifest }
                    : { kind: "npm", package: built, manifest };
            }),
        );
    },

    /** Read an image's builds hoisted by package name, as npm hoists: the root's links first. */
    async entries(
        value: BuildImage,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<Record<string, DependencyRelease>> {
        // keep the first build of each name, the root and its links coming first
        const entries: Record<string, DependencyRelease> = {};
        for (const entry of await BuildImage.releases(value, read)) {
            entries[entry.package.name] ??= entry;
        }

        return entries;
    },

    /** List every build an image has: its root's, then the others in digest order. */
    builds(value: BuildImage): Digest[] {
        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- an image keys its builds by digest
        const builds = Object.keys(value.builds) as Digest[];
        const others = builds.filter((digest) => digest !== value.root);

        return [value.root, ...others.toSorted()];
    },

    /** Report whether an image lists a build. */
    lists(value: BuildImage, build: Digest): boolean {
        return Object.hasOwn(value.builds, build);
    },
    /** Read the build each requirement of a listed build resolves to, by name. */
    links(value: BuildImage, build: Digest): Readonly<Record<string, Digest>> {
        return present(value.builds[build], `build '${build}' of image root '${value.root}'`);
    },

    /** Read the image of one of an image's builds: it and every build its links reach. */
    at(value: BuildImage, root: Digest): BuildImage {
        // walk from the build through the builds each reached build links
        const builds: Record<Digest, Readonly<Record<string, Digest>>> = {};
        const pending = [root];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            if (builds[next] === undefined) {
                const links = BuildImage.links(value, next);
                builds[next] = links;
                pending.push(...Object.values(links));
            }
        }

        return BuildImage.of(root, builds);
    },

    /** Layer an image's builds so each layer links only earlier layers. */
    layers(value: BuildImage): Digest[][] {
        // read each build's depth once, skipping edges back into a cycle
        const depths = new Map<Digest, number>();
        const visiting = new Set<Digest>();
        const depth = (manifest: Digest): number => {
            // answer a known depth, else walk the links
            const known = depths.get(manifest);
            if (known !== undefined) {
                return known;
            }
            visiting.add(manifest);
            const targets = Object.values(BuildImage.links(value, manifest)).filter(
                (each) => !visiting.has(each),
            );
            const below = targets.map((each) => depth(each));
            visiting.delete(manifest);
            const at = below.length === 0 ? 0 : Math.max(...below) + 1;
            depths.set(manifest, at);

            return at;
        };

        // group the builds by depth, the shallowest first
        const layers: Digest[][] = [];
        for (const manifest of BuildImage.builds(value)) {
            const at = depth(manifest);
            (layers[at] ??= []).push(manifest);
        }

        return layers;
    },

    /** Find the build a requirement of a build resolves to, absent for one the runtime links. */
    resolve(value: BuildImage, build: Digest, name: string): Digest | undefined {
        return BuildImage.links(value, build)[name];
    },
});

/** A root build and every build it runs with, each with the build its requirements resolve to. */
export type BuildImage = schema.Infer<typeof image>;
