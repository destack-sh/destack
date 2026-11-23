import type { Package } from "@destack/object";
import { PackageId } from "@destack/package";

/** A package the platform build takes from a repository on the universe's forge at a commit. */
export interface PlatformInput {
    /** The package's identifier and name, as its `destack.json` and `package.json` declare them. */
    readonly package: { readonly id: PackageId; readonly name: string };
    /** The repository on the universe's forge, such as `@destack/home`. */
    readonly repository: string;
    /** The commit the platform builds. */
    readonly commit: string;
    /** The package's directory in the repository. */
    readonly directory: string;
}

/** The packages the platform build takes from other repositories, as Nix flake inputs. */
export const PlatformInput = {
    /** Write the handle stacks install an input by, binding none of its resources or secrets. */
    handle(input: PlatformInput): Pick<Package, "id" | "name" | "resources" | "secrets"> {
        return { id: input.package.id, name: input.package.name, resources: {}, secrets: {} };
    },
};

/** Home: the front door of the universe and the hub of every person's space. */
export const HOME_INPUT: PlatformInput = {
    package: {
        id: PackageId.parse("package-01a205e0-f6ff-7bd0-9567-071511cd9f7c"),
        name: "@destack/home",
    },
    repository: "@destack/home",
    commit: "3262dcf7356605c084f10f2a127f4c97db4923ae",
    directory: ".",
};

/** The packages the platform build takes from other repositories. */
export const PLATFORM_INPUTS: readonly PlatformInput[] = [HOME_INPUT];
