import { isAbsolute, relative, resolve, sep } from "node:path";
import { PackageId, PackageName } from "@destack/package";
import { schema } from "@destack/schema";
import { GitAuthor } from "@destack/git";
import { Lease } from "@destack/resource";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** Where a checkout's working tree comes from. */
export const WorkingTreeSource = schema.discriminatedUnion("kind", [
    schema.object({
        /** Register the Git working tree already at the directory. */
        kind: schema.literal("existing"),
    }),
    schema.object({
        /** Initialize an empty Git repository at the directory. */
        kind: schema.literal("empty"),
    }),
    schema.object({
        /** Clone a remote Git repository into the directory. */
        kind: schema.literal("clone"),
        /** The remote Git repository. */
        remote: schema.string().min(1),
        /** The branch, tag or commit to check out, else the remote's default branch. */
        ref: schema.string().min(1).exactOptional(),
    }),
    schema.object({
        /** Track a new, empty repository and write a package from a published template into it. */
        kind: schema.literal("template"),
        /** The empty remote Git repository. */
        remote: schema.string().min(1),
        /** The template package. */
        template: PackageId,
        /** The template's release: a version or a tag. */
        release: schema.string().min(1),
        /** The name of the package made. */
        name: PackageName,
        /** The lease to write the repository, which its first commit is pushed with. */
        lease: Lease,
        /** The author of the first commit. */
        author: GitAuthor,
    }),
]);
/** Where a checkout's working tree comes from. */
export type WorkingTreeSource = schema.Infer<typeof WorkingTreeSource>;

/** The shape of a working tree: its canonical directory, Git root and origin. */
const shape = schema.object({
    /** The registered directory's absolute canonical path. */
    directory: schema.string(),
    /** The root of the directory's Git working tree. */
    root: schema.string(),
    /** The URL of the working tree's origin, null without one. */
    remote: schema.string().nullable(),
});
/** A Git working tree a directory belongs to: its canonical path and the tree's root. */
export type WorkingTree = schema.Infer<typeof shape>;

/** A Git working tree a directory belongs to, and the paths it normalizes. */
export const WorkingTree = Object.assign(shape, {
    /** Resolve a package directory in a working tree, normalized as installations record it. */
    package(
        root: string,
        directory: string,
    ): { readonly path: string; readonly directory: string } {
        // refuse a directory outside the working tree or on another drive
        const path = resolve(root, directory);
        const below = WorkingTree.below(root, path);
        if (below === undefined) {
            throw MachineError.of(
                "OUTSIDE_CHECKOUT_INVALID",
                "package directory is invalid: it is outside the checkout",
            );
        }

        return { path, directory: below };
    },

    /** Normalize an absolute path below a working tree's root as installations record it. */
    below(root: string, path: string): string | undefined {
        const below = relative(root, path);
        const isOutside = isAbsolute(below) || below === ".." || below.startsWith(`..${sep}`);

        return isOutside ? undefined : below === "" ? "." : below.split(sep).join("/");
    },
});
