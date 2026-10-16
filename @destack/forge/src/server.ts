import { defineServer } from "@destack/object/server";
import forgePackage from "./package.ts";
import { Forge } from "./server/forge.ts";

export * from "./server/index.ts";

/** The forge's server: repositories, packages and releases, with npm, builds and GitHub. */
export default defineServer(forgePackage, (context) => Forge.of(context).definition());
