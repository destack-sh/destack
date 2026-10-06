import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import { Command, CommandGroup, CommandItem, CommandList } from "@destack/ui/command";
import { Spinner } from "@destack/ui/spinner";
import { CommandReference } from "../declare/command.ts";
import type { JSX } from "../solid/component.ts";
import { Match, Switch } from "../solid/flow.ts";
import type { CommandEntry } from "../palette/entry.ts";
import { type PaletteStep, stepOf } from "./run.ts";

/** Render a command the shell launched, which runs once the person chooses it. */
export function ConfirmCommand(properties: {
    readonly command: CommandEntry;
    readonly onRun: (command: CommandEntry) => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Command shouldFilter={false}>
            <CommandList>
                <CommandGroup heading={locale.render(t`Run`)}>
                    <CommandItem
                        value={CommandReference.format(properties.command)}
                        onSelect={() => properties.onRun(properties.command)}
                    >
                        {properties.command.title}
                    </CommandItem>
                </CommandGroup>
            </CommandList>
        </Command>
    );
}

/** Render where a command's call stands: running, done or failed. */
export function Outcome(properties: { readonly step: PaletteStep }): JSX.Element {
    const locale = useLocale();

    return (
        <div data-slot="command-outcome" data-state={properties.step.kind}>
            <Switch>
                <Match when={properties.step.kind === "running"}>
                    <Spinner />
                </Match>
                <Match when={properties.step.kind === "done"}>{locale.render(t`Done`)}</Match>
                <Match when={stepOf(properties.step, "failed")}>
                    {(refused) => refused().message}
                </Match>
            </Switch>
        </div>
    );
}
