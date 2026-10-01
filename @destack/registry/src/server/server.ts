import type { CallKey } from "@destack/service/request";
import { type ObjectReference } from "@destack/sync";
import type { Resolver } from "@destack/account/directory";
import type { PackageStore } from "@destack/build/store";
import { and, eq, isNull, ne, type DatabaseConnection } from "@destack/db";
import { Call } from "@destack/object";
import { ObjectServer } from "@destack/object/server";
import { PackageName } from "@destack/package";
import type { DeclarationDescription } from "@destack/package/inspect";
import { Upgrade, Vocabulary } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import type {} from "@destack/package/manifest";
import { Digest, identifier, schema, Version } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { ServiceContext, ServiceImplementation } from "@destack/service/server";
import { validRange } from "semver";
import {
    dependency,
    LATEST_TAG,
    packageObject,
    release,
    tag,
    TagName,
    type Distribution,
    type PackageMetadata,
    type Release,
} from "../object/index.ts";
import { PackageArchive } from "../pack/index.ts";
import { registryService } from "../service/index.ts";
import { NpmServer } from "./npm.ts";

/** How long after publication a release may be unpublished, in milliseconds: npm's 72 hours. */
const UNPUBLISH_WINDOW_MILLISECONDS = 72 * 60 * 60 * 1000;

/** What a release's creation packed before its transaction. */
interface Packed {
    /** The version the build declares. */
    readonly version: string;
    /** The stored npm archive. */
    readonly distribution: Distribution;
    /** The npm declaration of the archive. */
    readonly metadata: PackageMetadata;
    /** The exact releases the archive requires. */
    readonly dependencies: readonly { readonly name: string; readonly version: string }[];
    /** The steps from the package's previous release, absent for its first. */
    readonly upgrade: Upgrade | undefined;
    /** The build's own declarations, with their terms. */
    readonly declarations: readonly DeclarationDescription[];
}

/** What a region serving the registry keeps and reaches. */
export interface RegistryServerOptions {
    /** The regional database with packages, releases, tags and dependencies. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The resolver of account handles and package names. */
    readonly resolver: Resolver;
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The URL npm clients reach the registry at. */
    readonly npm: URL;
}

/** Serves the registry's objects and the npm endpoints over them. */
export class RegistryServer {
    /** The object server executing the registry's methods. */
    readonly objects: ObjectServer<{
        readonly package: typeof packageObject;
        readonly release: typeof release;
        readonly tag: typeof tag;
        readonly dependency: typeof dependency;
    }>;
    /** The resolver of account handles with the directory that claims package names. */
    readonly resolver: Resolver;
    /** The store of builds and npm archives. */
    readonly store: PackageStore;
    /** The npm endpoints. */
    readonly npm: NpmServer;

    /** Serve the registry over a region's database and store. */
    constructor(options: RegistryServerOptions) {
        // publish releases by packing stored builds before the transaction, and keep tags on published releases
        this.resolver = options.resolver;
        this.store = options.store;
        const releases = release.handle({
            create: {
                prepare: (call) => this.#pack(call),
                effect: (call, next) => RegistryServer.#publish(call, next),
            },
            unpublish: (call) => RegistryServer.#unpublish(call),
        });
        const tags = tag.handle({
            create: (call, next) => RegistryServer.#createTag(call, next),
            update: (call, next) => RegistryServer.#updateTag(call, next),
            point: (call, next) => RegistryServer.#point(call, next),
        });

        // serve the objects in the caller's scope, claiming package names in the directory
        this.objects = new ObjectServer({
            objects: { package: packageObject, release: releases, tag: tags, dependency },
            database: options.database,
            callKey: options.callKey,
            origin: { package: registryService.package, service: registryService.name },
            directory: options.resolver.directory,
        });
        this.npm = new NpmServer(options.npm, this);
    }

    /** Implement the registry service with the npm endpoints. */
    service(): ServiceImplementation {
        return {
            ...this.objects.implement(registryService),
            route: (request, context) => this.npm.route(request, context),
        };
    }

    /** Find the package a global name names, reporting one the caller may not read as missing. */
    async find(name: string, context: ServiceContext): Promise<ObjectReference> {
        // read the named package as the caller
        const found = await this.#named(name);
        try {
            await this.objects.query(
                packageObject,
                "get",
                { accountId: found.scope, id: found.id },
                context,
            );
        } catch (error) {
            if (
                !(error instanceof ServiceError) ||
                (error.code !== "NOT_FOUND" && error.code !== "FORBIDDEN")
            ) {
                throw error;
            }
            throw new ServiceError("NOT_FOUND", { message: "package not found" });
        }

        return found;
    }

    /** Find the package with a global name `@<account handle>/<name>` through the directory. */
    async #named(name: string): Promise<ObjectReference> {
        // resolve the account handle, then the name within the account
        const [handle, local] = PackageName.parse(name).slice(1).split("/") as [string, string];
        const accountId = await this.resolver.account(handle);
        const found =
            accountId === undefined
                ? undefined
                : await packageObject.lookup(this.resolver.directory, "name", [local], accountId);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "package not found" });
        }

        return found;
    }

    /** Pack a stored build of the package under its global name, and store its npm archive. */
    async #pack(call: Call<typeof release.table>): Promise<Packed> {
        // refuse a tag a range reads, then pack the stored build
        RegistryServer.#requireTagName(RegistryServer.#tagName(call));
        const parentId = identifier("package").parse(call.input.parentId);
        const manifest = Digest.parse(call.input.manifest);
        const contents = await this.store.contents(manifest);
        const archive = await PackageArchive.pack(
            contents,
            schema.string().parse(call.input.commit),
        );
        if (archive.manifest !== manifest) {
            throw new ServiceError("BAD_REQUEST", {
                message: "archived manifest differs from the build",
            });
        }

        // require the build to be the package's, under the package's global name
        if (archive.packageId !== parentId) {
            throw new ServiceError("CONFLICT", { message: "the build is another package's" });
        }
        const named = await this.#named(archive.metadata.name);
        if (named.id !== parentId) {
            throw new ServiceError("CONFLICT", {
                message: "the npm name belongs to another package",
            });
        }

        // store the archive under its digest before any release refers to it
        await this.store.putFile(archive.file, async () => archive.open());

        // read the build's declarations and the upgrade it planned
        const upgrade = contents.manifest.upgrade;

        return {
            version: archive.metadata.version,
            distribution: archive.distribution,
            metadata: archive.metadata,
            dependencies: archive.dependencies,
            upgrade:
                upgrade === undefined
                    ? undefined
                    : await contents.reader.read(upgrade.file, Upgrade),
            declarations: await contents.reader.declarations(),
        };
    }

    /** Insert a release of an unused version, record its dependencies, and point its tag at it. */
    static async #publish(
        call: Call<typeof release.table>,
        next: (call?: Call<typeof release.table>) => Promise<unknown>,
    ): Promise<unknown> {
        // refuse a version the package took before, published or unpublished
        const packed = call.prepared as Packed;
        const parentId = identifier("package").parse(call.input.parentId);
        const versions = (
            await call.database
                .select({ version: release.table.version })
                .from(release.table)
                .where(eq(release.table.parentId, parentId))
        ).map((row) => row.version);
        if (versions.includes(packed.version)) {
            throw new ServiceError("CONFLICT", {
                message: "package version was already published",
            });
        }

        // require releases in order, planned from the latest
        const latest = versions.sort(Version.compare).at(-1);
        if (latest !== undefined && Version.compare(packed.version, latest) < 0) {
            throw new ServiceError("CONFLICT", {
                message: `publish releases in version order: ${packed.version} precedes ${latest}`,
            });
        } else if (packed.upgrade?.from !== latest) {
            throw new ServiceError("CONFLICT", {
                message:
                    latest === undefined
                        ? "the first release plans no upgrade"
                        : `build the release against ${latest}, the latest release`,
            });
        }

        // plan and advance the vocabulary
        const [owner] = await call.database
            .select({ vocabulary: packageObject.table.vocabulary })
            .from(packageObject.table)
            .where(eq(packageObject.table.id, parentId));
        try {
            Vocabulary.plan(owner!.vocabulary, packed.declarations);
        } catch (error) {
            throw error instanceof PlanError
                ? new ServiceError("CONFLICT", { message: error.message, cause: error })
                : error;
        }
        await call.invoke(packageObject, "advance", {
            id: parentId,
            vocabulary: Vocabulary.advance(owner!.vocabulary, packed.declarations, packed.version),
        });

        // insert the release
        const { version, distribution, metadata, upgrade } = packed;
        const created = await next(
            call.with({ input: { ...call.input, version, distribution, metadata, upgrade } }),
        );

        // record each exact release it requires
        const releaseId = Call.resultId(created)!;
        for (const required of packed.dependencies) {
            await call.invoke(dependency, "create", { parentId: releaseId, ...required });
        }

        // point its tag at it, latest unless the call names another
        await call.invoke(tag, "point", {
            parentId: call.input.parentId,
            name: RegistryServer.#tagName(call),
            version: packed.version,
        });

        return created;
    }

    /** Unpublish a release as npm allows, dropping its tags, and keep it as a tombstone. */
    static async #unpublish(call: Call<typeof release.table>): Promise<unknown> {
        // refuse releases already unpublished and releases past the window
        const unpublished = call.target!;
        if (unpublished.unpublishedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "release is already unpublished" });
        } else if (call.now - unpublished.createdAt >= UNPUBLISH_WINDOW_MILLISECONDS) {
            throw new ServiceError("CONFLICT", {
                message: "releases older than 72 hours stay published: deprecate them instead",
            });
        }

        // refuse while another package's published release requires it
        // NOTE #Incomplete: releases in other regions' databases may require it unseen
        const [dependent] = await call.database
            .select({ metadata: release.table.metadata })
            .from(dependency.table)
            .innerJoin(release.table, eq(release.table.id, dependency.table.parentId))
            .where(
                and(
                    eq(dependency.table.name, unpublished.metadata.name),
                    eq(dependency.table.version, unpublished.version),
                    ne(release.table.parentId, unpublished.parentId),
                    isNull(release.table.unpublishedAt),
                ),
            )
            .limit(1);
        if (dependent !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `${dependent.metadata.name}@${dependent.metadata.version} depends on this release`,
            });
        }

        // delete the tags pointing at it
        const pointing = await call.database
            .select({ id: tag.table.id, name: tag.table.name })
            .from(tag.table)
            .where(
                and(
                    eq(tag.table.parentId, unpublished.parentId),
                    eq(tag.table.version, unpublished.version),
                ),
            );
        for (const each of pointing) {
            await call.invoke(tag, "delete", { id: each.id });
        }

        // point latest at the highest version still published, when it pointed at this one
        const remaining = await call.database
            .select({ version: release.table.version })
            .from(release.table)
            .where(
                and(
                    eq(release.table.parentId, unpublished.parentId),
                    ne(release.table.version, unpublished.version),
                    isNull(release.table.unpublishedAt),
                ),
            );
        const [highest] = remaining
            .map((entry) => entry.version)
            .sort((left, right) => Version.compare(right, left));
        if (pointing.some((each) => each.name === LATEST_TAG) && highest !== undefined) {
            await call.invoke(tag, "point", {
                parentId: unpublished.parentId,
                name: LATEST_TAG,
                version: highest,
            });
        }

        return call.revise({ unpublishedAt: call.now });
    }

    /** Create a tag no range reads as one, pointing at a published release. */
    static async #createTag(
        call: Call<typeof tag.table>,
        next: (call?: Call<typeof tag.table>) => Promise<unknown>,
    ): Promise<unknown> {
        // require a tag name and a published target
        RegistryServer.#requireTagName(TagName.parse(call.input.name));
        await RegistryServer.#requirePublished(
            call.database,
            identifier("package").parse(call.input.parentId),
            schema.string().parse(call.input.version),
        );

        return next();
    }

    /** Move a tag only to a published release. */
    static async #updateTag(
        call: Call<typeof tag.table>,
        next: (call?: Call<typeof tag.table>) => Promise<unknown>,
    ): Promise<unknown> {
        // require a published target when the tag moves
        const version = schema.string().optional().parse(call.input.version);
        if (version !== undefined) {
            await RegistryServer.#requirePublished(call.database, call.target!.parentId, version);
        }

        return next();
    }

    /** Create a tag, or move the package's tag of the same name to the version. */
    static async #point(
        call: Call<typeof tag.table>,
        next: (call?: Call<typeof tag.table>) => Promise<unknown>,
    ): Promise<unknown> {
        // find the package's tag of the name
        const parentId = identifier("package").parse(call.input.parentId);
        const name = TagName.parse(call.input.name);
        const [existing] = await call.database
            .select({ id: tag.table.id })
            .from(tag.table)
            .where(and(eq(tag.table.parentId, parentId), eq(tag.table.name, name)));

        return existing === undefined
            ? next()
            : call.invoke(tag, "update", { id: existing.id, version: call.input.version });
    }

    /** Read the tag a release's creation names, latest when it names none. */
    static #tagName(call: Call<typeof release.table>): string {
        return TagName.optional().parse(call.input.tag) ?? LATEST_TAG;
    }

    /** Require a distribution tag name that no version range reads as a range. */
    static #requireTagName(name: string): void {
        if (validRange(name) !== null) {
            throw new ServiceError("BAD_REQUEST", { message: `invalid distribution tag: ${name}` });
        }
    }

    /** Require a version a tag may point at: a published release of the package. */
    static async #requirePublished(
        database: DatabaseConnection,
        packageId: Release["parentId"],
        version: string,
    ): Promise<void> {
        const [found] = await database
            .select({ id: release.table.id })
            .from(release.table)
            .where(
                and(
                    eq(release.table.parentId, packageId),
                    eq(release.table.version, version),
                    isNull(release.table.unpublishedAt),
                ),
            );
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: "distribution tag target is not published",
            });
        }
    }
}
