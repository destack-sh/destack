import { defineService } from "@destack/service";
import { dependency, packageObject, release, tag } from "../object/index.ts";

/** Packages, their releases, tags and dependencies, served as objects. */
export const registryService = defineService("registry", {
    objects: { package: packageObject, release, tag, dependency },
});
