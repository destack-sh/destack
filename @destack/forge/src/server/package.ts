import { schema, Version } from "@destack/schema";
import { PackageName } from "@destack/package";
import type { MethodContextOf, ResultOf } from "@destack/object";
import type { ObjectReference } from "@destack/sync";
import { packageObject, release, tag } from "@destack/registry/object";
import { setting } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { Forge } from "./forge.ts";
import { ReleaseMethod, type ReleaseOptions } from "./release.ts";

/** The package methods' handlers: publishing, resolving a global name and pointing a tag. */
const PackageMethod = {
    /** Find the package of a global name through the claims the forge follows on demand. */
    find(
        context: MethodContextOf<typeof packageObject, "resolve", ObjectReference | undefined>,
    ): Promise<ObjectReference> {
        return Forge.package(
            context.input.name,
            context,
            schema.identifier("space").parse(context.scope),
        );
    },

    /** Read the found package as the caller, hiding one the caller may not read. */
    async resolve(
        context: MethodContextOf<typeof packageObject, "resolve">,
    ): Promise<ResultOf<typeof packageObject, "resolve">> {
        return Forge.conceal("PACKAGE_NOT_FOUND", "package not found", () =>
            context.query(
                context.caller,
                packageObject.call("get", packageObject.target(context.prepared)),
            ),
        );
    },

    /** Read the version the next release of a package name takes, published yet or not. */
    async next(
        context: MethodContextOf<typeof packageObject, "next">,
    ): Promise<ResultOf<typeof packageObject, "next">> {
        // find the name's package, none before its first release, and its releases the caller reads
        const home = schema.identifier("space").parse(context.scope);
        const found = (await Forge.find(context.input.name, context, home)).package;
        const published =
            found === undefined
                ? []
                : await context.read(release).findMany({
                      where: { parentId: schema.identifier("package").parse(found.id) },
                  });

        // take its newest release, leaving its dev builds
        const [newest] = published
            .map((each) => each.version)
            .filter((version) => !Version.isPrerelease(version))
            .toSorted((left, right) => Version.compare(right, left));

        // start this month at its first release, and follow a release of this month or a later one
        const first = Version.first(context.now);
        const version =
            newest === undefined || Version.compare(newest, first) < 0
                ? first
                : Version.next(newest);

        return { version: Version.parse(version) };
    },

    /** Create each checked package once and insert its release, required ones first. */
    async publish(
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
    },

    /** Point a package's distribution tag at a version, creating the tag the first time. */
    async point(
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
            await context.mutate(context.caller, [
                tag.call("update", { id: existing.id, version }),
            ]);
        }

        return context.target;
    },
};

/** Implement packages of the forge's space. */
export function implementPackageObject(options: ReleaseOptions): typeof packageObject {
    return packageObject.implement({
        methods: {
            publish: {
                prepare: (context) => ReleaseMethod.prepare(context, options),
                handle: (context) => PackageMethod.publish(context),
            },
            resolve: {
                prepare: (context) => PackageMethod.find(context),
                handle: (context) => PackageMethod.resolve(context),
            },
            next: (context) => PackageMethod.next(context),
            point: (context) => PackageMethod.point(context),
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
                project: (packageId, { store }) =>
                    ReleaseMethod.retain(store, schema.identifier("package").parse(packageId)),
            },
        ],
    });
}
