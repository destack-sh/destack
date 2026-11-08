import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { Verb } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { openForge } from "./forge.ts";
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
            const { client, home } = await openForge(packageName, context);
            const found = await client.package.resolve({ spaceId: home, name: packageName });
            const { version } = await client.release.resolve({
                spaceId: home,
                parentId: found.id,
                release,
            });

            // point the tag at it
            const name = present(tag, "the tag argument");
            const { environment } = await context.native();
            const pointed = await client.package.point({
                spaceId: home,
                id: found.id,
                requestId: RequestId.create(environment),
                name,
                version,
            });
            await context.terminal.print(
                json ? JSON.stringify(pointed) : `Tagged ${packageName}@${version} ${name}.`,
            );
        },
    };
}
