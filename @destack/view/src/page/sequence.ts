import { type KeyPress, Shortcut } from "@destack/locale";
import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, type Setter } from "solid-js";

/** The time a person has for the next stroke, in milliseconds, half of VS Code's. */
export const STROKE_TIME = 2500;

/** The keys that only hold a stroke's modifiers. */
const MODIFIERS: ReadonlySet<string> = new Set(["Shift", "Control", "Alt", "Meta", "AltGraph"]);

/** What a shortcut runs. */
export interface ShortcutBinding<Target> {
    /** The shortcut. */
    readonly key: Shortcut;
    /** What it runs. */
    readonly target: Target;
}

/** The strokes a person typed of longer shortcuts, and what those run. */
export interface ShortcutProgress<Target = unknown> {
    /** The strokes typed, as the shortcuts write them. */
    readonly typed: string;
    /** The time of the last, as events tell it. */
    readonly at: number;
    /** The shortcuts that go on from them. */
    readonly next: readonly ShortcutBinding<Target>[];
}

/** The strokes a person presses after one another, matched against the shortcuts where they are. */
export class ShortcutSequence<Target> {
    /** What is typed of longer shortcuts, undefined between them. */
    readonly progress: Accessor<ShortcutProgress<Target> | undefined>;
    /** Replace what is typed. */
    readonly #setProgress: Setter<ShortcutProgress<Target> | undefined>;
    /** What is typed, ahead of the signal's next flush. */
    #current: ShortcutProgress<Target> | undefined;

    /** Start without strokes. */
    constructor() {
        // follow what is typed
        const [progress, setProgress] = createSignal<ShortcutProgress<Target> | undefined>(
            undefined,
            { ownedWrite: true },
        );
        this.progress = progress;
        this.#setProgress = setProgress;
    }

    /** Read a press: what it runs, and whether it is a stroke of some shortcut. */
    press(
        event: KeyPress & Pick<KeyboardEvent, "timeStamp">,
        bindings: readonly ShortcutBinding<Target>[],
        isCommand: boolean,
    ): { readonly target?: Target; readonly isTaken: boolean } {
        // wait while a modifier alone is down, and forget strokes typed too long ago
        if (isModifier(event)) {
            return { isTaken: false };
        }
        const current = this.#current;
        if (current !== undefined && event.timeStamp - current.at > STROKE_TIME) {
            this.cancel();
        }

        // find the shortcuts that go on from what is typed with this stroke
        const typed = this.#current?.typed;
        const prefix = typed === undefined ? [] : Shortcut.strokes(typed);
        const matches = bindings.flatMap((binding) => {
            // compare the strokes typed, then this one
            const strokes = Shortcut.strokes(binding.key);
            const stroke = strokes[prefix.length];
            const isNext =
                stroke !== undefined &&
                prefix.every((each, index) => each === strokes[index]) &&
                Shortcut.matches(stroke, event, isCommand);

            return isNext ? [{ binding, strokes }] : [];
        });
        const longer = matches.filter(({ strokes }) => strokes.length > prefix.length + 1);
        const exact = matches.find(({ strokes }) => strokes.length === prefix.length + 1);
        const [first] = longer;

        // wait for the next stroke of a longer one
        if (first !== undefined) {
            this.#show({
                typed: first.strokes.slice(0, prefix.length + 1).join(" "),
                at: event.timeStamp,
                next: longer.map(({ binding }) => binding),
            });

            return { isTaken: true };
        }
        // run the one it completes
        else if (exact !== undefined) {
            this.cancel();

            return { target: exact.binding.target, isTaken: true };
        }

        return { isTaken: false };
    }

    /** Report whether strokes of a longer shortcut are typed. */
    isTyping(): boolean {
        return this.#current !== undefined;
    }

    /** Forget the strokes typed. */
    cancel(): void {
        this.#show(undefined);
    }

    /** Take what is typed, writing no signal on a server. */
    #show(progress: ShortcutProgress<Target> | undefined): void {
        this.#current = progress;
        if (!isServer) {
            this.#setProgress(() => progress);
        }
    }
}

/** Report whether a press only holds a modifier. */
export function isModifier(event: KeyPress): boolean {
    return MODIFIERS.has(event.key);
}
