import { resolve } from "node:path";
import { UniverseHome } from "@destack/machine/process";
import { PackageName } from "@destack/package";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CommandOption, type CommandBody } from "../command/command.ts";
import { PackageSpecifier } from "../catalog/installation.ts";

/** The template a package is made from unless another is asked for. */
const DEFAULT_TEMPLATE = "@destack/app";

/** Make a package from a published template in a new repository, checked out on this computer. */
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
        // read the package's name and the template's release
        const name = PackageName.parse(present(named, "the package argument"));
        const template = PackageSpecifier.release(
            CommandOption.string(options["template"], "template") ?? DEFAULT_TEMPLATE,
        );

        // find the template at its forge
        const templates = await context.registry(template.name);
        const found = await templates.client.package.resolve({
            spaceId: templates.home,
            name: template.name,
        });

        // create the package's repository in its account, at its universe's address
        const device = await context.native();
        const forge = await context.registry(name);
        const repository = await forge.client.repository.create({
            spaceId: forge.home,
            accountId: forge.accountId,
            requestId: RequestId.create(device.environment),
            name: PackageName.split(name).local,
            hosting: "platform",
        });
        const launcher = await context.launcher();
        const { universe } = launcher.session;
        const remote = new URL(`/${name}.git`, universe).href;
        const lease = await forge.client.repository.push({
            spaceId: forge.home,
            id: repository.id,
        });

        // check the repository out under the checkouts directory, else where asked, with the package committed in it
        const target =
            directory === undefined || directory === ""
                ? UniverseHome.current(context.environment).checkout(remote)
                : resolve(directory);
        const created = await device.machine().checkout.create({
            machineId: (await device.loopback.computer()).machineId,
            requestId: RequestId.create(device.environment),
            directory: target,
            source: {
                kind: "template",
                remote,
                template: found.id,
                release: template.release,
                name,
                lease,
                author: await launcher.author(),
            },
        });
        const { terminal } = context;
        const lines = [
            terminal.event("Created", "done", `package '${name}'`, created.directory),
            terminal.hint("next", `cd ${created.directory} && destack dev`),
        ];
        await context.terminal.print(
            json
                ? JSON.stringify({ name, directory: created.directory, checkout: created.id })
                : lines.join("\n"),
        );
    },
};
