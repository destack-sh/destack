import { Installation, installation } from "./installation.ts";
import { StackDefinition } from "../declare/index.ts";
import { type DatabaseConnection, eq, Snapshot, type Select } from "@destack/db";
import { defineObject, field, type ObjectOf } from "@destack/object";
import { PackageId, PackageBuild } from "@destack/package";
import { type BuildReader } from "@destack/package/manifest";
import { Digest, type DigestCrypto, Identifier, schema, type Uuid, Version } from "@destack/schema";
import { build, Build } from "./build.ts";
import { space } from "@destack/account/object";
import { StackEvaluation } from "./stack.ts";
import { StackError } from "../error/index.ts";
import { Binding } from "./binding.ts";
import { argument, plural, t } from "@destack/locale";

/** The revision namespace: the DNS namespace's UUIDv8 of installation-revision.destack.app. */
const REVISION_NAMESPACE: Uuid = "472ed1a0-20d1-8e8e-b9d9-c6879123e146";

/** An immutable evaluation of an installation's selected build. */
export const installationRevision = defineObject(
    {
        name: "installation-revision",
        plural: "installationRevisions",
        title: t`${plural(argument("count"), { one: "Installation revision", other: "Installation revisions" })}`,
        scope: space,
        fields: {
            /** The evaluated installation. */
            installation: field.reference(
                (): ObjectOf<{ identity: "installation" }> => installation,
                {
                    delete: "cascade",
                },
            ),
            /** The space's build evaluated: compiled here or pulled as a release. */
            build: field.reference((): ObjectOf<{ identity: "build" }> => build, {
                delete: "restrict",
            }),
            /** The installable packages its build depends on, at the builds its image has. */
            dependencies: field.json(schema.array(PackageBuild)),
            /** The state each declaration requires, such as main's tables, by name. */
            states: field.blobMap(),
            /** The release of the build's package, which the next upgrade plans from. */
            release: field.string(Version),
            /** The build's declarations as a blob, which the next upgrade compares against. */
            declarations: field.blob(),
            /** The evaluated stack definition, for stacks. */
            definition: field.json(StackDefinition).optional(),
            /** The targets its package's declarations run with, by package and declaration name. */
            bindings: field.json(schema.array(Binding)),
            /** The installation's generation the revision was made at. */
            generation: field.integer(),
            /** The SHA-256 digest of the build, the stack evaluation's inputs and the bindings. */
            digest: field.string(Digest),
        },
        constraints: {
            installationId: { kind: "unique", on: ["installationId", "id"] },
            digest: { kind: "unique", on: ["installationId", "digest"] },
            digestForm: {
                kind: "check",
                condition: { digest: { length: 64, characters: "0-9a-f" } },
            },
        },
        access: {
            permissions: ["read", "record"],
        },
    },
    (method) => ({
        create: method.create("record", {
            visibility: "package",
            fields: [
                "installationId",
                "buildId",
                "dependencies",
                "states",
                "release",
                "declarations",
                "definition",
                "bindings",
                "generation",
                "digest",
            ],
            input: schema.object({
                /** The package whose build the revision evaluates. */
                packageId: PackageId,
            }),
        }),
    }),
);

/** An immutable evaluation applied to an installation. */
export type InstallationRevision = Select<typeof installationRevision.table>;

/** What a revision records of its build: dependencies, states, release and declarations. */
export type RevisionDescription = Pick<
    InstallationRevision,
    "dependencies" | "states" | "release" | "declarations"
>;

/** The revisions of an installation: one per distinct build and stack evaluation. */
export const InstallationRevision = {
    /** Read the build row a revision evaluated. */
    async build(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
    ): Promise<Select<typeof build.table> & { readonly image: Digest }> {
        // read the revision, refusing an unknown one
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        if (revision === undefined) {
            throw StackError.of("REVISION_NOT_FOUND", `revision '${revisionId}' not found`);
        }

        return await Build.require(snapshot, revision.scope, revision.buildId);
    },

    /** Read the build a revision of a package evaluated from the space's builds. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
        packageId: PackageId,
        builds: { readonly root: (packageId: PackageId, image: Digest) => Promise<BuildReader> },
    ): Promise<BuildReader> {
        const built = await InstallationRevision.build(snapshot, revisionId);

        return builds.root(packageId, built.image);
    },

    /** Read the revision an installation follows, absent before its first. */
    async followed(
        database: DatabaseConnection,
        target: Pick<Installation, "targetRevisionId">,
    ): Promise<InstallationRevision | undefined> {
        if (target.targetRevisionId === null) {
            return undefined;
        }
        const [revision] = await database
            .select()
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, target.targetRevisionId));

        return revision;
    },

    /** Digest a build's image and its stack evaluation's inputs, which decide its definition. */
    digest(
        image: Digest,
        inputs: Omit<StackEvaluation, "definition"> | undefined,
        bindings: readonly Binding[],
        crypto: DigestCrypto,
    ): Promise<Digest> {
        return Digest.json(
            inputs === undefined
                ? { image, bindings }
                : {
                      image,
                      export: inputs.export,
                      parameters: inputs.parameters,
                      settings: inputs.settings,
                      routes: inputs.routes,
                      bindings,
                  },
            crypto,
        );
    },

    /** Identify an installation's revision of a digest, the same everywhere, as git its objects. */
    id(
        installationId: Identifier<"installation">,
        digest: Digest,
        crypto: DigestCrypto,
    ): Promise<Identifier<"installation-revision">> {
        const name = `${installationId}/${digest}`;

        return Identifier.hash("installation-revision", REVISION_NAMESPACE, name, crypto);
    },
};
