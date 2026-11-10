import { Stage } from "@destack/update/release";
import { Link } from "@destack/desktop/link";
import { window } from "@destack/desktop/object";
import { present } from "@destack/schema";
import { type Verb, OptionValue } from "../command/command.ts";

import type { Context } from "../context/context.ts";
import { Application } from "@destack/desktop/workload";
import { CliError } from "../error/index.ts";
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

            // open its page on its origin in the browser, which signs the person in at the universe
            if (options["browser"] === true) {
                const person = await context.person();
                const native = await context.native();
                const { url } = await native.launch(person, address);
                await context.terminal.printDiagnostic(`Opening ${url}`);
                await context.environment.opener.openUrl(url);
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
