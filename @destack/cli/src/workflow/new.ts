import { resolve } from "node:path";
import { PackageName } from "@destack/package";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { type Verb, OptionValue } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { openForge } from "./forge.ts";
import { readSpecifier } from "./install.ts";

/** The template a package is made from unless another is asked for. */
const DEFAULT_TEMPLATE = "@destack/template-blank";

/** Make a package from a published template in a new checkout on this computer. */
export function newCommand(context: Context): Verb {
    return {
        description: "Make a package from a template in a new directory, as a checkout",
        arguments: [
            { name: "package", usage: "@handle/name" },
            { name: "directory", default: "", isDirectory: true },
        ],
        options: {
            template: {
                type: "string",
                description: `The template's package, ${DEFAULT_TEMPLATE} by default`,
                argument: "@scope/name[@version|@tag]",
            },
        },
        json: true,
        async run({ arguments: { package: named, directory }, options, json }) {
            // read the package's name, its directory and the template's release
            const name = PackageName.parse(present(named, "the package argument"));
            const target = resolve(
                directory === undefined || directory === ""
                    ? PackageName.split(name).local
                    : directory,
            );
            const template = readSpecifier(
                OptionValue.string(options["template"], "template") ?? DEFAULT_TEMPLATE,
            );

            // find the template at its forge
            const { client, home } = await openForge(template.name, context);
            const found = await client.package.resolve({ spaceId: home, name: template.name });

            // make the package in a new checkout of this computer
            const device = await context.native();
            const created = await device.machine().checkout.create({
                machineId: (await device.loopback.computer()).machineId,
                requestId: RequestId.create(device.environment),
                directory: target,
                source: { kind: "template", template: found.id, release: template.release, name },
            });
            const lines = [
                `Created ${name} in ${created.directory}.`,
                "",
                `  cd ${created.directory}`,
                "  bun install",
                "  destack dev",
            ];
            await context.terminal.print(
                json
                    ? JSON.stringify({ name, directory: created.directory, checkout: created.id })
                    : lines.join("\n"),
            );
        },
    };
}
