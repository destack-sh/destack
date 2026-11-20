import { resolve } from "node:path";
import { PackageName } from "@destack/package";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CommandOption, type CommandBody } from "../command/command.ts";
import { readSpecifier } from "./install.ts";

/** The template a package is made from unless another is asked for. */
const DEFAULT_TEMPLATE = "@destack/blank";

/** Make a package from a published template in a new checkout on this computer. */
export const newCommand: CommandBody = {
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
    isJson: true,
    async run({ arguments: { package: named, directory }, options, isJson: json }, context) {
        // read the package's name, its directory and the template's release
        const name = PackageName.parse(present(named, "the package argument"));
        const target = resolve(
            directory === undefined || directory === "" ? PackageName.split(name).local : directory,
        );
        const template = readSpecifier(
            CommandOption.string(options["template"], "template") ?? DEFAULT_TEMPLATE,
        );

        // find the template at its forge
        const { client, home } = await context.registry(template.name);
        const found = await client.package.resolve({ spaceId: home, name: template.name });

        // make the package in a new checkout of this computer
        const device = await context.native();
        const created = await device.machine().checkout.create({
            machineId: (await device.loopback.computer()).machineId,
            requestId: RequestId.create(device.environment),
            directory: target,
            source: { kind: "template", template: found.id, release: template.release, name },
        });
        const { terminal } = context;
        const lines = [
            terminal.event("Created", "done", `package '${name}'`, created.directory),
            terminal.hint("next", `cd ${created.directory} && bun install && destack dev`),
        ];
        await context.terminal.print(
            json
                ? JSON.stringify({ name, directory: created.directory, checkout: created.id })
                : lines.join("\n"),
        );
    },
};
