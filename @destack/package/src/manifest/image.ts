import type { Crypto } from "@destack/environment";
import { canonicalize, Digest, present, schema, Version } from "@destack/schema";
import type { DependencyRelease } from "../definition/dependency.ts";
import { DependencyName } from "../definition/package.ts";
import { PackageManifest } from "./manifest.ts";
import type { ImportMap } from "./output.ts";
import type { Runtime } from "../runtime/index.ts";

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

    /** Make an image with some builds replaced by others, in its builds and its links. */
    relinked(value: BuildImage, replaced: ReadonlyMap<Digest, Digest>): BuildImage {
        const replace = (digest: Digest) => replaced.get(digest) ?? digest;
        const builds = Object.fromEntries(
            Object.entries(value.builds).map(([digest, links]) => [
                replace(Digest.parse(digest)),
                Object.fromEntries(
                    Object.entries(links).map(([name, target]) => [name, replace(target)]),
                ),
            ]),
        );

        return BuildImage.of(replace(value.root), builds);
    },

    /** Make an image of the same builds on another root build, linking what the root linked. */
    rooted(value: BuildImage, root: Digest): BuildImage {
        const { [value.root]: links = {}, ...others } = value.builds;

        return BuildImage.of(root, { ...others, [root]: links });
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

    /** Read the builds a runtime loads of an image, from what its root runs or every export. */
    async loads(
        value: BuildImage,
        runtime: Runtime,
        read: (manifest: Digest) => Promise<PackageManifest | undefined>,
        entries: "run" | "export" = "run",
    ): Promise<ReadonlySet<Digest>> {
        // start at each output's workloads and the exports its views and frame mount, or at every export
        const manifests = new Map<Digest, Promise<PackageManifest | undefined>>();
        const once = (digest: Digest) => {
            const reading = manifests.get(digest) ?? read(digest);
            manifests.set(digest, reading);

            return reading;
        };
        const first = await once(value.root);
        const pending = PackageManifest.outputs(
            present(first, `the root of image ${value.root}`),
            runtime,
        ).flatMap((output) => {
            const mounted = new Set(
                [
                    ...Object.values(output.views),
                    ...(output.frame === undefined ? [] : [output.frame]),
                ].map((view) => view.entrypoint),
            );
            const started =
                entries === "export"
                    ? Object.keys(output.exports)
                    : [
                          ...Object.values(output.workloads).map((each) => each.entrypoint),
                          ...Object.entries(output.exports)
                              .filter(([, exported]) => mounted.has(exported.path))
                              .map(([subpath]) => subpath),
                      ];

            return started.map((subpath) => ({ manifest: value.root, subpath }));
        });

        // follow each export once to the image's builds its chunks import
        const loaded = new Set<Digest>([value.root]);
        const visited = new Set<string>();
        const expanded = new Set<Digest>();
        const units: Digest[] = [];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const key = `${next.manifest} ${next.subpath}`;
            const opened = visited.has(key) ? undefined : await once(next.manifest);
            visited.add(key);
            if (opened !== undefined) {
                // follow every export of the root
                const output = present(
                    PackageManifest.outputs(opened, runtime)[0],
                    `the ${runtime} output of ${next.manifest}`,
                );
                if (
                    entries === "export" &&
                    next.manifest === value.root &&
                    !expanded.has(next.manifest)
                ) {
                    expanded.add(next.manifest);
                    for (const subpath of Object.keys(output.exports)) {
                        pending.push({ ...next, subpath });
                    }
                }

                // take each imported package's build
                const { imports } = present(
                    output.exports[next.subpath],
                    `the export ${next.subpath} of ${next.manifest}`,
                );
                for (const specifier of imports) {
                    const build = BuildImage.resolve(
                        value,
                        next.manifest,
                        DependencyName.of(specifier),
                    );
                    const imported = build === undefined ? undefined : await once(build);
                    // take an npm unit's build with its links later
                    if (
                        build !== undefined &&
                        imported !== undefined &&
                        !PackageManifest.isPackage(imported)
                    ) {
                        units.push(build);
                    }
                    // follow a package's build through the export it imports
                    else if (build !== undefined) {
                        loaded.add(build);
                        pending.push({
                            manifest: build,
                            subpath: DependencyName.subpath(specifier),
                        });
                    }
                }
            }
        }

        // add each imported npm unit with the units it links
        for (let unit = units.pop(); unit !== undefined; unit = units.pop()) {
            if (!loaded.has(unit)) {
                loaded.add(unit);
                units.push(...Object.values(BuildImage.links(value, unit)));
            }
        }

        return loaded;
    },

    /** Map an image's modules on a runtime below a URL prefix, builtins at their linked builds. */
    async imports(
        value: BuildImage,
        runtime: Runtime,
        base: string,
        linked: ReadonlyMap<string, BuildImage>,
        read: (manifest: Digest) => Promise<PackageManifest>,
        entries: "run" | "export" = "run",
    ): Promise<ImportMap> {
        // read the builds the root's modules import on the runtime, from what it runs or from every export
        const loaded = await BuildImage.loads(value, runtime, read, entries);
        const links = [...linked.values()].flatMap((each) => Object.entries(each.builds));
        const builtins = BuildImage.of(value.root, Object.fromEntries(links));

        // read the npm units the linked builtins link, by package name, which the root's side takes when they meet its range
        const platform = new Map<string, { readonly digest: Digest; readonly version: string }[]>();
        for (const [build] of links) {
            const digest = Digest.parse(build);
            const manifest = await read(digest);
            if (!PackageManifest.isPackage(manifest)) {
                const { name, version } = manifest.package;
                platform.set(name, [...(platform.get(name) ?? []), { digest, version }]);
            }
        }

        // map each dependency's exports in its dependent's scope
        const map: ImportMap = { imports: {}, scopes: {} };
        const visited = new Set<string>([`${value.root} app`]);
        const pending = [{ digest: value.root, scope: map.imports, isLinked: false }];
        for (const { digest, scope, isLinked } of pending) {
            const dependent = await read(digest);
            const own = dependent.package.name;
            for (const output of PackageManifest.outputs(dependent, runtime)) {
                // take each imported package's build
                const names = new Set(
                    Object.values(output.exports).flatMap((exported) =>
                        exported.imports.map((specifier) => DependencyName.of(specifier)),
                    ),
                );
                const dependencies = [...names].flatMap((name) => {
                    // take the build itself for its package's name, as a package imports its exports by it
                    if (name === own) {
                        return [{ name, manifest: digest, isLinked }];
                    }

                    // take the platform's build of an npm package it links at a version the dependent accepts, so the page loads one copy
                    const range = dependent.requires[name];
                    const unit =
                        isLinked || range === undefined
                            ? undefined
                            : platform
                                  .get(name)
                                  ?.find((each) => Version.meets(each.version, range));
                    if (unit !== undefined) {
                        return [{ name, manifest: unit.digest, isLinked: true }];
                    }

                    // take the image's build of a name
                    const pinned = isLinked ? undefined : BuildImage.resolve(value, digest, name);
                    const isPinned = pinned !== undefined && loaded.has(pinned);
                    const manifest = isPinned
                        ? pinned
                        : ((BuildImage.lists(builtins, digest)
                              ? BuildImage.resolve(builtins, digest, name)
                              : undefined) ?? linked.get(name)?.root);

                    return manifest === undefined ? [] : [{ name, manifest, isLinked: !isPinned }];
                });
                for (const dependency of dependencies) {
                    // map the dependency's exports to its files below its build's prefix
                    const { name, manifest } = dependency;
                    const prefix = `${base}${manifest}/`;
                    for (const each of PackageManifest.outputs(await read(manifest), runtime)) {
                        for (const [subpath, exported] of Object.entries(each.exports)) {
                            const specifier = subpath === "." ? name : `${name}${subpath.slice(1)}`;
                            scope[specifier] = `${prefix}${exported.path}`;
                        }
                    }

                    // map the dependency's own imports in its scope once per side
                    const side = `${manifest} ${dependency.isLinked ? "linked" : "app"}`;
                    if (!visited.has(side)) {
                        visited.add(side);
                        pending.push({
                            digest: manifest,
                            scope: (map.scopes[prefix] ??= {}),
                            isLinked: dependency.isLinked,
                        });
                    }
                }
            }
        }

        return map;
    },
});

/** A root build and every build it runs with, each with the build its requirements resolve to. */
export type BuildImage = schema.Infer<typeof image>;
