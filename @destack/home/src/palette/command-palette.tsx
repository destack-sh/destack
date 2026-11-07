import { Command } from "@destack/ui/command";
import { CommandReference } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import { SchemaForm } from "@destack/ui/schema-form";
import { type JSX, Match, Switch } from "@destack/view";
import type {
    ObjectEntry,
    PaletteFocus,
    RegisteredEntry,
    ViewEntry,
    WindowEntry,
} from "@destack/view/palette";
import type { PaletteListing } from "../command/listing.ts";
import { ApproveCommand, ConfirmCommand, Outcome } from "./outcome.tsx";
import { ObjectPicker } from "./picker.tsx";
import { PaletteRun, stepOf } from "./run.ts";
import { SearchStep } from "./search.tsx";

/** The properties of a command palette. */
export interface CommandPaletteProperties {
    /** The entries of the person's spaces. */
    readonly listing: PaletteListing;
    /** What the window the palette opens over shows. */
    readonly focus: PaletteFocus;
    /** A command to offer first, which runs once the person confirms it. */
    readonly command?: CommandReference;
    /** Open a view, or an object in the view presenting it. */
    readonly onOpen: (entry: ViewEntry | ObjectEntry) => void;
    /** Bring a window of this computer into focus. */
    readonly onWindow: (entry: WindowEntry) => void;
    /** Run a command of the view under the palette in its page with its input. */
    readonly onRegistered: (entry: RegisteredEntry, input: JsonObject) => void;
    /** Close the palette. */
    readonly onClose: () => void;
}

/** Render the command palette: search the person's entries, then run a command. */
export function CommandPalette(properties: CommandPaletteProperties): JSX.Element {
    // offer the command asked for
    const { listing, focus } = properties;
    const running = new PaletteRun(listing, focus, {
        close: properties.onClose,
        run: properties.onRegistered,
    });
    const asked = listing.entries.find(
        (entry) =>
            (entry.kind === "command" || entry.kind === "registered") &&
            properties.command !== undefined &&
            CommandReference.format(entry) === CommandReference.format(properties.command),
    );
    if (asked?.kind === "command" || asked?.kind === "registered") {
        running.offer(asked);
    }

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
            <PaletteSteps
                listing={listing}
                focus={focus}
                running={running}
                onOpen={properties.onOpen}
                onWindow={properties.onWindow}
            />
        </div>
    );
}

/** Render the step a palette's run stands at. */
function PaletteSteps(properties: {
    readonly listing: PaletteListing;
    readonly focus: PaletteFocus;
    readonly running: PaletteRun;
    readonly onOpen: (entry: ViewEntry | ObjectEntry) => void;
    readonly onWindow: (entry: WindowEntry) => void;
}): JSX.Element {
    const { listing, running } = properties;

    return (
        <Switch>
            {/* Search */}
            <Match when={running.step().kind === "search"}>
                <SearchStep
                    listing={listing}
                    focus={properties.focus}
                    running={running}
                    onOpen={properties.onOpen}
                    onWindow={properties.onWindow}
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
                            catalog={listing.catalog(picked().command)}
                            command={picked().command}
                            onObject={(object) => running.pick(picked().command, [object])}
                        />
                    </Command>
                )}
            </Match>

            {/* Filling a command's input */}
            <Match when={stepOf(running.step(), "input")}>
                {(filled) => (
                    <SchemaForm
                        schema={filled().parameters}
                        submit={filled().command.title}
                        onSubmit={(input) =>
                            running.submit(filled().command, input, filled().targets)
                        }
                    />
                )}
            </Match>

            {/* Approving a command removing its targets */}
            <Match when={stepOf(running.step(), "approve")}>
                {(approved) => (
                    <ApproveCommand
                        command={approved().command}
                        count={approved().targets.length}
                        onApprove={() =>
                            running.call(approved().command, approved().input, approved().targets)
                        }
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
