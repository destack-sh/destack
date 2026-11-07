import { present } from "@destack/schema";
import type { Verb } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { Forge } from "./forge.ts";
import { readSpecifier } from "./install.ts";

/** Point a package's distribution tag at one of its releases, as its publisher. */
export function tagCommand(context: Context): Verb {
    return {
        description: "Point a package's distribution tag at one of its releases",
        arguments: [
            { name: "package", usage: "@scope/name@version" },
            { name: "tag", usage: "name" },
        ],
        json: true,
        async run({ arguments: { package: specifier, tag }, json }) {
            // resolve the release the specifier names in its package's forge
            const { name: packageName, release } = readSpecifier(
                present(specifier, "the package argument"),
            );
            const forge = await Forge.open(packageName, context);
            const resolved = await forge.resolve(packageName, release);

            // point the tag at it
            const name = present(tag, "the tag argument");
            const { version } = resolved.release;
            const pointed = await forge.point(resolved.packageId, name, version);
            await context.terminal.print(
                json ? JSON.stringify(pointed) : `Tagged ${packageName}@${version} ${name}.`,
            );
        },
    };
}
