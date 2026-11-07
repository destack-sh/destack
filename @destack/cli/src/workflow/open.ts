import { Stage } from "@destack/update/release";
import { Link } from "@destack/desktop/link";
import { window } from "@destack/desktop/object";
import { present } from "@destack/schema";
import { type Verb, OptionValue } from "../command/command.ts";

import type { Context } from "../context/context.ts";
import { Application } from "@destack/desktop/workload";
import { browse } from "../output/index.ts";
import { CliError } from "../error/index.ts";
import type { GrantPermission } from "@destack/access";
import { MachineError } from "@destack/machine/error";
import type { Session } from "@destack/machine/device";
import type { Address } from "@destack/view/declare";

/** Open a link, or an object by its identifier, in a desktop window or the browser. */
export function openCommand(context: Context): Verb {
    return {
        description:
            "Open a link, or an object by its identifier, in a desktop window or the browser",
        arguments: [{ name: "target", usage: "link|id" }],
        json: true,
        options: {
            view: {
                type: "string",
                description: "The view to open, the strongest presenting the object by default",
                argument: "name",
            },
            browser: { type: "boolean", description: "Open in the browser" },
        },
        async run({ arguments: { target: given }, options, json }) {
            // read the link of this stage's desktop, or the object of an identifier in the space
            const target = present(given, "the target argument");
            const links = Link.of(Stage.current(context.environment));
            const view = OptionValue.string(options["view"], "view");
            const address = URL.canParse(target)
                ? links.parse(target)
                : await objectAddress(target, view, context);
            if (address === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `${target} is not a view or object link under ${links.scheme}://`,
                );
            }

            // launch it on its origin in the browser, asking the person to grant what its view asks
            if (options["browser"] === true) {
                const person = await context.person();
                const { url } = await person.launch(address).catch(async (error: unknown) => {
                    if (!MachineError.is(error, "GRANT_REQUIRED")) {
                        throw error;
                    }
                    await openGrant(person, address, error.data.permissions, context);

                    return person.launch(address);
                });
                await context.terminal.printDiagnostic(`Opening ${url}`);
                await browse(url, context.environment);
                await context.terminal.print(json ? JSON.stringify({ url }) : `Opened ${url}.`);
            }
            // open it as a window of the running desktop, else launch the desktop with it
            else {
                const link = links.format(address);
                const desktop = new Application((await context.native()).home, context.environment);
                if (await desktop.isRunning()) {
                    await using stack = new AsyncDisposableStack();
                    const windows = await context.objects({ window }, stack);
                    await windows.mutate(window).open({ link }).confirmed;
                } else {
                    await desktop.launch(desktop.locate(), link);
                }
                await context.terminal.print(json ? JSON.stringify({ link }) : `Opened ${link}.`);
            }
        },
    };
}

/** Read the address of an object of the space by its identifier, in a view or the strongest. */
async function objectAddress(
    id: string,
    view: string | undefined,
    context: Context,
): Promise<Address> {
    const catalog = await context.catalog();
    const space = await (await context.person()).address(await context.requireSpace());

    return catalog.select(id, space, view, context);
}

/** Ask the person on the terminal to grant a view what it asks, refusing to open it otherwise. */
async function openGrant(
    person: Pick<Session, "grant">,
    address: Address,
    permissions: readonly GrantPermission[],
    context: Context,
): Promise<void> {
    // list each permission as its verb and type
    const name = "object" in address ? address.space : address.installation;
    const listed = [...new Set(permissions.map((entry) => `${entry.name} ${entry.type}`))];
    await context.terminal.printDiagnostic(`${name} asks to act for you: ${listed.join(", ")}`);

    // grant it once the person allows
    if (!(await context.terminal.prompt.confirm(`Allow ${name}?`))) {
        throw CliError.of("GRANT_REFUSED", `${name} was not granted what it asks`);
    }
    await person.grant(address);
}
