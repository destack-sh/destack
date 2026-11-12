import type { Crypto } from "@destack/environment";
import { canonicalize, Digest, present, schema } from "@destack/schema";
import type { DependencyRelease } from "../definition/dependency.ts";
import { DependencyName } from "../definition/package.ts";
import type { PackageManifest } from "./manifest.ts";

/** The lock format a store writes. */
export const LOCK_FORMAT = 2;

/** A root build and every build it runs with, each with the build its requirements resolve to. */
const lock = schema
    .object({
        /** The lock format version. */
        formatVersion: schema.literal(LOCK_FORMAT),
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
                    message: `build '${target}' is missing: the lock resolves to it`,
                });
            }
        }
    });

/** A root build and every build it runs with, each with the build its requirements resolve to. */
export const Lock = Object.assign(lock, {
    /** Lock a root build with the builds it runs with, the root linking none unless given. */
    of(
        root: Digest,
        builds: Readonly<Record<Digest, Readonly<Record<string, Digest>>>> = {},
    ): Lock {
        return { formatVersion: LOCK_FORMAT, root, builds: { [root]: {}, ...builds } };
    },

    /** Lock a root with one build per package, hoisted as npm does, resolving each by name. */
    async hoisted(
        root: Digest,
        builds: Readonly<Record<string, Digest>>,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<Lock> {
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

        return Lock.of(root, links);
    },

    /** Serialize a lock as its canonical bytes, whose digest identifies it. */
    serialize(value: Lock): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(canonicalize(lock.parse(value)));
    },

    /** Take the digest of a lock, its address in every store. */
    digest(value: Lock, crypto: Crypto): Promise<Digest> {
        return Digest.of(Lock.serialize(value), crypto);
    },

    /** Read every build of a lock with its package: the root's, its links, then the rest. */
    async releases(
        value: Lock,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<DependencyRelease[]> {
        // order the root and its own links first, so their names come before nested ones
        const linked = Object.values(Lock.links(value, value.root));
        const ordered = [...new Set([value.root, ...linked, ...Lock.builds(value)])];

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

    /** Read a lock's builds hoisted by package name, as npm hoists: the root's links first. */
    async entries(
        value: Lock,
        read: (digest: Digest) => Promise<PackageManifest>,
    ): Promise<Record<string, DependencyRelease>> {
        // keep the first build of each name, the root and its links coming first
        const entries: Record<string, DependencyRelease> = {};
        for (const entry of await Lock.releases(value, read)) {
            entries[entry.package.name] ??= entry;
        }

        return entries;
    },

    /** List every build a lock has: its root's, then the others in digest order. */
    builds(value: Lock): Digest[] {
        const others = Object.keys(value.builds).filter((digest) => digest !== value.root);

        return [value.root, ...others.toSorted().map((digest) => Digest.parse(digest))];
    },

    /** Report whether a lock lists a build. */
    lists(value: Lock, build: Digest): boolean {
        return Object.hasOwn(value.builds, build);
    },
    /** Read the build each requirement of a listed build resolves to, by name. */
    links(value: Lock, build: Digest): Readonly<Record<string, Digest>> {
        return present(value.builds[build], `build '${build}' of lock root '${value.root}'`);
    },

    /** Read the lock of one of a lock's builds: it and every build its links reach. */
    at(value: Lock, root: Digest): Lock {
        // walk from the build through the builds each reached build links
        const builds: Record<Digest, Readonly<Record<string, Digest>>> = {};
        const pending = [root];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            if (builds[next] === undefined) {
                const links = Lock.links(value, next);
                builds[next] = links;
                pending.push(...Object.values(links));
            }
        }

        return Lock.of(root, builds);
    },

    /** Layer a lock's builds so each layer links only earlier layers. */
    layers(value: Lock): Digest[][] {
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
            const targets = Object.values(Lock.links(value, manifest)).filter(
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
        for (const manifest of Lock.builds(value)) {
            const at = depth(manifest);
            (layers[at] ??= []).push(manifest);
        }

        return layers;
    },

    /** Find the build a requirement of a build resolves to, absent for one the runtime links. */
    resolve(value: Lock, build: Digest, name: string): Digest | undefined {
        return Lock.links(value, build)[name];
    },
});

/** A root build and every build it runs with, each with the build its requirements resolve to. */
export type Lock = schema.Infer<typeof lock>;
