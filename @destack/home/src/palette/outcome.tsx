import { argument, plural, t } from "@destack/locale";
import { Command, CommandGroup, CommandItem, CommandList } from "@destack/ui/command";
import { Spinner } from "@destack/ui/spinner";
import { CommandReference } from "@destack/object";
import { type JSX, Match, Switch, useLocale } from "@destack/view";
import type { CommandEntry } from "@destack/view/palette";
import { type PaletteStep, type RunEntry, stepOf } from "./run.ts";

/** Render a command the shell launched, which runs once the person chooses it. */
export function ConfirmCommand(properties: {
    readonly command: RunEntry;
    readonly onRun: (command: RunEntry) => void;
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

/** Render a command removing its targets, which runs once the person approves it. */
export function ApproveCommand(properties: {
    readonly command: CommandEntry;
    readonly count: number;
    readonly onApprove: () => void;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Command shouldFilter={false}>
            <CommandList>
                <CommandGroup
                    heading={locale.render(
                        t`${plural(argument("count"), { one: "Remove # object?", other: "Remove # objects?" })}`,
                        { count: Math.max(properties.count, 1) },
                    )}
                >
                    <CommandItem
                        value={CommandReference.format(properties.command)}
                        onSelect={() => properties.onApprove()}
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
                {/* Running */}
                <Match when={properties.step.kind === "running"}>
                    <Spinner />
                </Match>

                {/* Done */}
                <Match when={properties.step.kind === "done"}>{locale.render(t`Done`)}</Match>

                {/* Failed */}
                <Match when={stepOf(properties.step, "failed")}>
                    {(refused) => refused().message}
                </Match>
            </Switch>
        </div>
    );
}
