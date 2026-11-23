import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { CommandBody } from "../command/command.ts";
import { PackageSpecifier } from "./installation.ts";

/** Point a package's distribution tag at one of its releases, as its publisher. */
export const tagCommand: CommandBody = {
    arguments: [
        { name: "package", usage: "@scope/name@version" },
        { name: "tag", usage: "name" },
    ],
    isJson: true,
    async run({ arguments: { package: specifier, tag }, isJson: json }, context) {
        // resolve the release the specifier names in its package's forge
        const { name: packageName, release } = PackageSpecifier.release(
            present(specifier, "the package argument"),
        );
        const { client, home } = await context.registry(packageName);
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
            json
                ? JSON.stringify(pointed)
                : context.terminal.event(
                      "Tagged",
                      "done",
                      `release '${packageName}@${version}'`,
                      name,
                  ),
        );
    },
};
