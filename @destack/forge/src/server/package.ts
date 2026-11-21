import { type BuildStore } from "@destack/build/store";
import { BuildImage, PackageManifest } from "@destack/package/manifest";
import { type DependencyRelease, type PackageId, PackageName } from "@destack/package";
import type { MethodContextOf, ResultOf } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Snapshot } from "@destack/db";
import { type Digest, type Identifier, present, schema, Version } from "@destack/schema";
import { type ObjectReference, RequestId } from "@destack/sync";
import { NpmTarball } from "@destack/registry/npm";
import {
    LATEST_TAG,
    packageObject,
    type PackageVisibility,
    release,
    tag,
    TagName,
} from "@destack/registry/object";
import { setting, SettingValue } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { ForgeError } from "../error/index.ts";
import { Forge } from "./forge.ts";

/** The package file every build carries, whose `publishConfig` asks its access. */
const PACKAGE_FILE = "package.json";

/** The visibility each of npm's access levels publishes a package at. */
const NPM_ACCESS = { public: "public", restricted: "private" } as const satisfies Record<
    string,
    PackageVisibility
>;

/** The part of a package file asking its access, as npm reads it at publish. */
const PackageAccess = schema.looseObject({
    /** The settings npm publishes with. */
    publishConfig: schema
        .looseObject({
            /** Whether the package is public or restricted to its owners. */
            access: schema.enum(["public", "restricted"]).exactOptional(),
        })
        .exactOptional(),
});

/** A package a publish releases, checked before its transaction. */
type Publication = MethodContextOf<typeof packageObject, "publish">["prepared"][number];

/** Implement packages of the forge's space. */
export function implementPackageObject(store: BuildStore): typeof packageObject {
    return packageObject.implement({
        methods: {
            publish: {
                prepare: (context) => prepare(context, store),
                handle: publish,
            },
            resolve: {
                prepare: find,
                handle: resolve,
            },
            next,
            point,
        },
        projections: [
            {
                // delete a package's prereleases past the space's retention
                name: "expire",
                watches: [
                    { table: release.table, on: { id: "parentId" } },
                    { table: tag.table, on: { id: "parentId" } },
                    { table: setting.table, where: { name: buildRetention.name }, isListing: true },
                ],
                project: (packageId, { store: objects }) =>
                    expire(objects, schema.identifier("package").parse(packageId)),
            },
        ],
    });
}

/** Check and copy up each of the account's packages in a pushed image, required ones first. */
async function prepare(
    context: MethodContextOf<typeof packageObject, "publish", readonly Publication[] | undefined>,
    store: BuildStore,
): Promise<readonly Publication[]> {
    // refuse a tag a range reads, and lease the pushed image through the publish
    TagName.require(context.input.tag ?? LATEST_TAG);
    const { image } = context.input;
    const request = present(context.requestId, "the release's request");
    const until = new Date(RequestId.expiry(request, context.now));
    await store.lease(image, until);

    // require a pushed image
    if (!(await store.contains(image))) {
        throw ForgeError.of("BUILD_NOT_FOUND", `image '${image}' not found`);
    }
    const read = await store.image(image);

    // take the packages of the root's handle within the image, required ones first
    const root = PackageManifest.package(await store.get(read.root));
    const { handle } = PackageName.split(root.name);
    const owned: { readonly digest: Digest; readonly manifest: PackageManifest }[] = [];
    for (const digest of BuildImage.layers(read).flat()) {
        const manifest = await store.get(digest);
        if (
            PackageManifest.isPackage(manifest) &&
            PackageName.split(manifest.package.name).handle === handle
        ) {
            owned.push({ digest, manifest });
        }
    }

    // check each in its own image, skipping a required one already released, as pnpm's recursive publish does
    const publishing = new Set<string>();
    const candidates: Publication[] = [];
    for (const { digest, manifest } of owned) {
        // leave a required package's version released before alone, tags included
        const required = PackageManifest.package(manifest);
        const [taken] = await context.read(release).findMany({
            where: { parentId: required.id, version: required.version },
            limit: 1,
        });
        if (digest !== read.root && taken !== undefined) {
            continue;
        }

        // check the root and each unreleased required one, counting it as released for those after it
        const at = digest === read.root ? image : await store.putImage(BuildImage.at(read, digest));
        await store.lease(at, until);
        const candidate = await check(context, store, at, publishing);
        publishing.add(`${candidate.package.id}@${candidate.package.version}`);
        candidates.push(candidate);
    }

    return candidates;
}
/** Check and copy up one package's image before the publish's transaction. */
async function check(
    context: MethodContextOf<typeof packageObject, "publish", readonly Publication[] | undefined>,
    store: BuildStore,
    image: Digest,
    publishing: ReadonlySet<string>,
): Promise<Publication> {
    // read the image's root build
    const { accountId, commit } = context.input;
    const read = await store.image(image);
    const contents = await store.contents(read.root);

    // require the root build's graph, which every release carries for its readers
    if (!(await store.isIndexed(read.root))) {
        throw ForgeError.of(
            "BUILD_INVALID",
            `image '${image}' is invalid: its root build carries no graph`,
        );
    }

    // refuse a release of uncommitted changes that is no prerelease
    if (commit === undefined && !Version.isPrerelease(contents.manifest.package.version)) {
        throw ForgeError.of(
            "BUILD_INVALID",
            `image '${image}' is invalid: a release of uncommitted changes must be a prerelease`,
        );
    }

    // require the package's name to be the publishing account's, and not another package's
    const released = PackageManifest.package(contents.manifest);
    const { id, name, version } = released;
    const home = schema.identifier("space").parse(context.scope);
    const named = await Forge.find(name, context, home);
    if (named.accountId !== accountId) {
        throw ForgeError.of(
            "BUILD_INVALID",
            `image '${image}' is invalid: its root is a build of '${name}', not of the publishing account`,
        );
    } else if (named.package !== undefined && named.package.id !== id) {
        throw ForgeError.of("NPM_NAME_EXISTS", "npm name already exists");
    }

    // refuse a builtin outside the platform's organization
    if (contents.manifest.role === "builtin") {
        await Forge.requireBuiltin(name, context, home);
    }

    // refuse a locked package hosted here, the versions this publish releases counting as released
    const entries = await store.entriesOf(read);
    await requirePublished(context, id, entries, publishing);

    // answer the release of this version again when another image of the same build published it
    const [taken] = await context
        .read(release)
        .findMany({ where: { parentId: id, version }, limit: 1 });
    const isRepeated =
        taken !== undefined &&
        taken.image !== image &&
        (await store.image(taken.image)).root === read.root;

    // copy up the image and every build it lists that the host's distribution ships
    const builds = Object.values(entries).map((entry) => ({
        kind: "manifest" as const,
        digest: entry.manifest,
    }));
    await store.copyUp([{ kind: "image", digest: image }, ...builds]);

    // pack the tarball npm clients download once, and store it for the release to refer to
    const { bytes, listed } = await NpmTarball.pack(store, image, context.environment);
    const tarball = await store.blobs.write(
        (async function* () {
            yield bytes;
        })(),
    );

    // read the visibility the package file asks through npm's access
    const authored = PackageAccess.parse(
        JSON.parse(new TextDecoder().decode(await contents.load(PACKAGE_FILE))),
    );
    const access = authored.publishConfig?.access;

    return {
        package: { id, name, version },
        ...(access === undefined ? {} : { visibility: NPM_ACCESS[access] }),
        release: {
            image: isRepeated ? taken.image : image,
            ...(commit === undefined ? {} : { commit }),
            version,
            role: contents.manifest.role,
            declarations: await contents.declarations(),
            npm: listed,
            tarball,
        },
    };
}
/** Refuse a hosted locked package whose locked version is unreleased or yanked, like Cargo. */
async function requirePublished(
    context: MethodContextOf<typeof packageObject, "publish", readonly Publication[] | undefined>,
    root: PackageId,
    entries: Readonly<Record<string, DependencyRelease>>,
    publishing: ReadonlySet<string>,
): Promise<void> {
    // read the hosted dependencies and their releases in two reads
    const dependencies = Object.values(entries).flatMap((entry) =>
        entry.kind === "destack" && entry.package.id !== root ? [entry.package] : [],
    );
    const hosted = await context
        .read(packageObject)
        .findMany({ where: { id: { in: dependencies.map((each) => each.id) } } });
    const released = await context
        .read(release)
        .findMany({ where: { parentId: { in: hosted.map((each) => each.id) } } });
    const releases = new Map(released.map((each) => [`${each.parentId}@${each.version}`, each]));

    // refuse a hosted dependency whose locked version is yanked or not released, by this publish or before
    const ids = new Set(hosted.map((each) => each.id));
    for (const dependency of dependencies.filter((each) => ids.has(each.id))) {
        const key = `${dependency.id}@${dependency.version}`;
        const found = releases.get(key);
        const locked = `${dependency.name}@${dependency.version}`;

        // refuse a version yanked, which new dependents skip
        if (found !== undefined && found.yankedAt !== null) {
            throw ForgeError.of(
                "DEPENDENCY_BLOCKED",
                `cannot release with dependency '${locked}': it is yanked`,
            );
        }
        // refuse a version neither released nor in this publish
        else if (found === undefined && !publishing.has(key)) {
            throw ForgeError.of(
                "DEPENDENCY_BLOCKED",
                `cannot release with dependency '${locked}': it is not published`,
            );
        }
    }
}
/** Create each checked package once and insert its release, required ones first. */
async function publish(
    context: MethodContextOf<typeof packageObject, "publish">,
): Promise<ResultOf<typeof packageObject, "publish">> {
    // publish each package in the order its checks ran, required ones first
    const { accountId, tags = [] } = context.input;
    const pointed = context.input.tag;
    for (const published of context.prepared) {
        // create the package the first time, readable as asked, else as its package file asks
        const { id, name, version } = published.package;
        const visibility = context.input.visibility ?? published.visibility;
        await context.mutate(context.caller, [
            packageObject.call("create", {
                id,
                accountId,
                name: PackageName.split(name).local,
                role: published.release.role,
                ...(visibility === undefined ? {} : { visibility }),
            }),
        ]);

        // insert the release, which advances the package's role and vocabulary
        await context.mutate(context.self, [
            release.call("create", {
                parentId: id,
                ...published.release,
                ...(pointed === undefined ? {} : { tag: pointed }),
            }),
        ]);

        // point each further tag at it
        await context.mutate(
            context.caller,
            tags.map((each) => packageObject.call("point", { id, name: each, version })),
        );
    }

    return {
        releases: context.prepared.map((published) => ({
            package: published.package,
            image: published.release.image,
        })),
    };
}
/** Find the package of a global name through the claims the forge follows on demand. */
function find(
    context: MethodContextOf<typeof packageObject, "resolve", ObjectReference | undefined>,
): Promise<ObjectReference> {
    return Forge.package(
        context.input.name,
        context,
        schema.identifier("space").parse(context.scope),
    );
}
/** Read the found package as the caller, hiding one the caller may not read. */
async function resolve(
    context: MethodContextOf<typeof packageObject, "resolve">,
): Promise<ResultOf<typeof packageObject, "resolve">> {
    return Forge.conceal("PACKAGE_NOT_FOUND", "package not found", () =>
        context.query(
            context.caller,
            packageObject.call("get", packageObject.target(context.prepared)),
        ),
    );
}
/** Read the version the next release of a package name takes, published yet or not. */
async function next(
    context: MethodContextOf<typeof packageObject, "next">,
): Promise<ResultOf<typeof packageObject, "next">> {
    // find the name's package, none before its first release, and its newest release the caller reads
    const home = schema.identifier("space").parse(context.scope);
    const found = (await Forge.find(context.input.name, context, home)).package;
    const newest =
        found === undefined
            ? undefined
            : await Forge.newest(schema.identifier("package").parse(found.id), context);

    // start this month at its first release, and follow a release of this month or a later one
    const first = Version.first(context.now);
    const version =
        newest === undefined || Version.compare(newest, first) < 0 ? first : Version.next(newest);

    return { version: Version.parse(version) };
}
/** Point a package's distribution tag at a version, creating the tag the first time. */
async function point(
    context: MethodContextOf<typeof packageObject, "point">,
): Promise<ResultOf<typeof packageObject, "point">> {
    // find the package's tag of the name
    const { name, version } = context.input;
    const parentId = context.target.id;
    const [existing] = await context.read(tag).findMany({ where: { parentId, name } });

    // create the tag
    if (existing === undefined) {
        await context.mutate(context.caller, [tag.call("create", { parentId, name, version })]);
    }
    // move the tag
    else {
        await context.mutate(context.caller, [tag.call("update", { id: existing.id, version })]);
    }

    return context.target;
}
/** Delete a package's prereleases past the build retention, keeping the newest and tagged. */
async function expire(store: ObjectStore, packageId: Identifier<"package">): Promise<void> {
    // read how many prereleases of each package the space keeps
    const { database, scope } = store;
    const snapshot = Snapshot.live(database);
    const kept = await SettingValue.resolve(snapshot, buildRetention, { scope });
    const space = schema.identifier("space").parse(scope);
    const releases = await snapshot
        .read(release.table)
        .findMany({ where: { scope: space, parentId: packageId } });
    const tagged = new Set(
        (
            await snapshot
                .read(tag.table)
                .findMany({ where: { scope: space, parentId: packageId } })
        ).map((each) => `${each.parentId} ${each.version}`),
    );

    // drop each package's untagged prereleases past the newest it keeps
    const prereleases = releases
        .filter((each) => Version.isPrerelease(each.version))
        .toSorted((left, right) => right.createdAt - left.createdAt);
    const dropped = [...Map.groupBy(prereleases, (each) => each.parentId).values()].flatMap(
        (newestFirst) =>
            newestFirst
                .slice(kept)
                .filter((each) => !tagged.has(`${each.parentId} ${each.version}`)),
    );

    // delete them
    if (dropped.length > 0) {
        await store.mutate(
            store.as(release),
            {
                id: RequestId.create(store.environment),
                calls: dropped.map((each) => release.call("delete", release.target(each))),
            },
            { component: "retention" },
        );
    }
}
