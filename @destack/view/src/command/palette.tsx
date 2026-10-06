import { Command } from "@destack/ui/command";
import { CommandReference } from "../declare/command.ts";
import { SchemaForm } from "../form/schema.tsx";
import type { JSX } from "../solid/component.ts";
import { Match, Switch } from "../solid/flow.ts";
import { createSignal } from "../solid/reactive.ts";
import type {
    CommandEntry,
    PaletteClient,
    PaletteFocus,
    PaletteListing,
} from "../palette/entry.ts";
import { ConfirmCommand, Outcome } from "./outcome.tsx";
import { ObjectPicker } from "./picker.tsx";
import { PaletteRun, stepOf } from "./run.ts";
import { SearchStep } from "./search.tsx";

/** The properties of a command palette. */
export interface CommandPaletteProperties {
    /** What the palette reaches as the person. */
    readonly client: PaletteClient;
    /** What the window the palette opens over shows. */
    readonly focus: PaletteFocus;
    /** A command to offer first, such as one its keybinding chose, which runs once the person confirms it. */
    readonly command?: CommandReference;
}

/** Render the command palette: search the person's commands, views, windows, spaces and recent objects, then run a command, picking its object and filling its input. */
export function CommandPalette(properties: CommandPaletteProperties): JSX.Element {
    // load the listing, offering the command asked for once it arrives
    const [listing, setListing] = createSignal<PaletteListing>({ entries: [], failures: [] });
    const { client, focus } = properties;
    const running = new PaletteRun(client, focus);
    client.list().then((loaded) => {
        setListing(loaded);
        const asked = loaded.entries.find(
            (entry): entry is CommandEntry =>
                entry.kind === "command" &&
                properties.command !== undefined &&
                CommandReference.format(entry) === CommandReference.format(properties.command),
        );
        if (asked !== undefined) {
            running.offer(asked);
        }
    }, reportError);

    return (
        <div
            data-slot="command-palette"
            onKeyDown={(event) => {
                // widen a narrowed search or close on Escape
                if (event.key === "Escape") {
                    running.escape();
                }
            }}
        >
            <PaletteSteps client={client} focus={focus} running={running} listing={listing()} />
        </div>
    );
}

/** Render the step a palette's run stands at. */
function PaletteSteps(properties: {
    readonly client: PaletteClient;
    readonly focus: PaletteFocus;
    readonly running: PaletteRun;
    readonly listing: PaletteListing;
}): JSX.Element {
    const { client, running } = properties;

    return (
        <Switch>
            {/* Search */}
            <Match when={running.step().kind === "search"}>
                <SearchStep
                    client={client}
                    focus={properties.focus}
                    running={running}
                    listing={properties.listing}
                />
            </Match>

            {/* Confirming a launched command */}
            <Match when={stepOf(running.step(), "confirm")}>
                {(confirmed) => (
                    <ConfirmCommand
                        command={confirmed().command}
                        onRun={(command) => running.run(command)}
                    />
                )}
            </Match>

            {/* Picking a command's object */}
            <Match when={stepOf(running.step(), "object")}>
                {(picked) => (
                    <Command shouldFilter={false}>
                        <ObjectPicker
                            client={client}
                            command={picked().command}
                            onObject={(object) =>
                                running.proceed(picked().command, picked().shape, object)
                            }
                        />
                    </Command>
                )}
            </Match>

            {/* Filling a command's input */}
            <Match when={stepOf(running.step(), "input")}>
                {(filled) => (
                    <SchemaForm
                        schema={filled().shape.input}
                        submit={filled().command.title}
                        onSubmit={(input) => running.call(filled().command, input, filled().object)}
                    />
                )}
            </Match>

            {/* The outcome */}
            <Match when={true}>
                <Outcome step={running.step()} />
            </Match>
        </Switch>
    );
}
