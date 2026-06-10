import { For, Show, createMemo, createSignal } from "solid-js";

/// One executable entry in the command bar.
export type Command = {
    id: string;
    label: string;
    /// Short annotation shown right-aligned, like a shortcut or category.
    hint?: string;
    run: () => void;
};

/// One ranked row: a workspace file or a command.
type CommandItem = {
    key: string;
    label: string;
    hint: string;
    score: number;
    run: () => void;
};

const maximumItems = 12;

type CommandBarProps = {
    commands: readonly Command[];
    files: readonly string[];
    onClose: () => void;
    onOpenFile: (path: string) => void;
};

export function CommandBar(props: CommandBarProps) {
    const [query, setQuery] = createSignal("");
    const [index, setIndex] = createSignal(0);

    const items = createMemo<readonly CommandItem[]>(() => {
        const needle = query().trim().toLowerCase();

        // files rank against their full path, commands against their label
        const files = props.files.flatMap((path) => {
            const score = fuzzyScore(needle, path.toLowerCase());
            return score == undefined
                ? []
                : [
                      {
                          key: `file:${path}`,
                          label: path,
                          hint: "open",
                          score,
                          run: () => props.onOpenFile(path),
                      },
                  ];
        });
        const commands = props.commands.flatMap((command) => {
            const score = fuzzyScore(needle, command.label.toLowerCase());
            return score == undefined
                ? []
                : [
                      {
                          key: `command:${command.id}`,
                          label: command.label,
                          hint: command.hint ?? "command",
                          score: score + 1,
                          run: command.run,
                      },
                  ];
        });

        return [...commands, ...files].sort((a, b) => b.score - a.score).slice(0, maximumItems);
    });

    const select = (item: CommandItem) => {
        props.onClose();
        item.run();
    };
    const navigate = (event: KeyboardEvent) => {
        const all = items();

        // cycle selection, run on enter, dismiss on escape
        if (event.key === "ArrowDown") {
            event.preventDefault();
            setIndex((value) => Math.min(value + 1, all.length - 1));
        } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setIndex((value) => Math.max(value - 1, 0));
        } else if (event.key === "Enter") {
            event.preventDefault();
            const item = all[Math.min(index(), all.length - 1)];
            if (item != undefined) {
                select(item);
            }
        } else if (event.key === "Escape") {
            event.preventDefault();
            props.onClose();
        }
    };

    return (
        <div
            class="absolute inset-0 z-40 grid justify-center bg-black/30 pt-14"
            onClick={props.onClose}
        >
            <div
                class="h-fit w-[34rem] max-w-[calc(100vw-2rem)] border border-destack-frame bg-editor-window text-sm shadow-2xl"
                onClick={(event) => event.stopPropagation()}
            >
                {/* Query input */}
                <input
                    class="w-full border-b border-editor-line bg-transparent px-3 py-2 text-editor-text outline-none placeholder:text-editor-muted/60"
                    onInput={(event) => {
                        setQuery(event.currentTarget.value);
                        setIndex(0);
                    }}
                    onKeyDown={navigate}
                    placeholder="search files and commands"
                    ref={(element) => requestAnimationFrame(() => element.focus())}
                    spellcheck={false}
                    value={query()}
                />

                {/* Ranked results */}
                <Show
                    fallback={<div class="px-3 py-2 text-editor-muted">no matches</div>}
                    when={items().length > 0}
                >
                    <ul class="max-h-80 overflow-auto py-1">
                        <For each={items()}>
                            {(item, position) => (
                                <li>
                                    <button
                                        class="flex w-full min-w-0 items-baseline justify-between gap-3 px-3 py-1 text-left"
                                        classList={{
                                            "bg-destack-accent/15 text-editor-text":
                                                position() === index(),
                                            "text-editor-muted hover:text-editor-text":
                                                position() !== index(),
                                        }}
                                        onClick={() => select(item)}
                                        onMouseMove={() => setIndex(position())}
                                        type="button"
                                    >
                                        <span class="min-w-0 truncate">{item.label}</span>
                                        <span class="shrink-0 text-xs text-editor-muted/70 lowercase">
                                            {item.hint}
                                        </span>
                                    </button>
                                </li>
                            )}
                        </For>
                    </ul>
                </Show>
            </div>
        </div>
    );
}

/// Subsequence match score, higher is better, undefined when not matched.
function fuzzyScore(needle: string, target: string): number | undefined {
    if (needle === "") {
        return 0;
    }

    // walk the target consuming needle characters in order
    let score = 0;
    let position = 0;
    for (const character of needle) {
        const found = target.indexOf(character, position);
        if (found < 0) {
            return undefined;
        }

        // contiguous and word-start hits score higher than scattered ones
        const previous = target[found - 1];
        const isWordStart = found === 0 || previous === "/" || previous === " " || previous === ".";
        score += found === position ? 3 : isWordStart ? 2 : 1;
        position = found + 1;
    }

    // shorter targets win ties
    return score + 10 / (10 + target.length);
}
