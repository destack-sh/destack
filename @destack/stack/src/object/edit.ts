import { PackageDirectory, InstallationName } from "@destack/object";
import { DependencyName } from "@destack/package";
import { Commit, schema } from "@destack/schema";
import { GitAuthor } from "@destack/git";
import { defineWorkflow } from "@destack/workflow";
import { t } from "@destack/locale";

/** The branch an edit moves by one commit, and the package directory it changes there. */
export const EditSource = schema.object({
    /** The registered repository, in its forge's home space. */
    repository: schema.object({
        /** The home space the repository's row lives in. */
        scope: schema.string().min(1),
        /** The repository. */
        id: schema.identifier("repository"),
    }),
    /** The branch, such as main, without refs/heads/. */
    branch: schema.string().min(1),
    /** The package directory within the repository. */
    directory: PackageDirectory,
});
/** The branch an edit moves. */
export type EditSource = schema.Infer<typeof EditSource>;

/** One change to a package's source, applied to its manifest or its declarations. */
export const EditOperation = schema.discriminatedUnion("kind", [
    schema.object({
        /** Add a dependency to the package's manifest, or change its range. */
        kind: schema.literal("dependency.add"),
        /** The dependency's name. */
        package: DependencyName,
        /** The range the package accepts, such as latest or 2026.11.0. */
        range: schema.string().min(1),
    }),
    schema.object({
        /** Remove a dependency from the package's manifest. */
        kind: schema.literal("dependency.remove"),
        /** The dependency's name. */
        package: DependencyName,
    }),
    schema.object({
        /** Install a package in one of the package's stacks. */
        kind: schema.literal("installation.add"),
        /** The export of the stack. */
        stack: schema.string().min(1),
        /** The installation's name in the stack. */
        name: InstallationName,
        /** The installed package, imported from its `package` export. */
        package: DependencyName,
    }),
    schema.object({
        /** Remove an installation from one of the package's stacks. */
        kind: schema.literal("installation.remove"),
        /** The export of the stack. */
        stack: schema.string().min(1),
        /** The installation's name in the stack. */
        name: InstallationName,
    }),
]);
/** One change to a package's source. */
export type EditOperation = schema.Infer<typeof EditOperation>;

/** What an edit changes: a branch's package, by its operations in order. */
export const EditInput = schema.object({
    /** The branch moved. */
    source: EditSource,
    /** The changes, applied in order. */
    operations: schema.array(EditOperation).min(1),
    /** The message of the commit. */
    message: schema.string().min(1),
    /** The person the commit names as its author, the run's actor. */
    author: GitAuthor,
});
/** What an edit changes. */
export type EditInput = schema.Infer<typeof EditInput>;

/** What an edit did: the branch's commit it applied to, and the commit it pushed. */
export const EditOutput = schema.object({
    /** The branch's commit the edit applied to. */
    base: Commit,
    /** The commit the edit pushed. */
    commit: Commit,
});
/** What an edit did. */
export type EditOutput = schema.Infer<typeof EditOutput>;

/** Commit a change to a branch's package as its actor, checked on a runner before it pushes. */
export const edit = defineWorkflow({
    name: "edit",
    title: t`Edit`,
    input: EditInput,
    output: EditOutput,
});
