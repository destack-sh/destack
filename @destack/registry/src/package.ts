import { t } from "@destack/locale";
import { defineMount, definePackage } from "@destack/object";
import { RegistryError } from "./error/error.ts";
import { packageObject, reference, release, repository, tag } from "./object/index.ts";

/** The registry protocol: packages, releases, tags, repositories, references and pushed builds. */
export default definePackage({
    title: t`Registry`,
    description: t`Publish and resolve packages, releases and tags, and push builds and Git, through the registry a space installs.`,
    icon: "package",
    color: "green",
    objects: [repository, reference, packageObject, release, tag],
    mounts: [defineMount({ name: "builds", path: "/builds" })],
    errors: RegistryError,
});
