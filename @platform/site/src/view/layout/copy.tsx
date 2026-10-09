import { Icon } from "@destack/icon";
import check from "@destack/icon/phosphor/check";
import copy from "@destack/icon/phosphor/copy";
import * as style from "@destack/style";
import { type Accessor, createSignal, type JSX, merge, omit, onCleanup, Show } from "@destack/view";
import { writeClipboard } from "@destack/view/primitives/clipboard";
import { makeTimer } from "@destack/view/primitives/timer";
import { Button, type ButtonProperties } from "@destack/ui/button";
import { visuallyHiddenStyle } from "@destack/ui/visually-hidden";

/** How long a copy shows as copied, in milliseconds, long enough to read a check mark. */
const COPIED_DURATION = 2000;

/** The size and variant of a copy button unless set. */
const DEFAULTS = { size: "icon-sm", variant: "ghost" } as const;

/** A copy to the clipboard that shows as copied for a moment. */
export interface Copy {
    /** Write text to the clipboard, rejecting when the browser refuses. */
    readonly copy: (text: string) => Promise<void>;
    /** Whether the last copy is showing. */
    readonly isCopied: Accessor<boolean>;
}

/** Create a copy to the clipboard that shows each copy as copied for a moment. */
export function createCopy(): Copy {
    // hold the shown state and the timer that resets it
    const [isCopied, setIsCopied] = createSignal(false, { ownedWrite: true });
    let stop: (() => void) | undefined;
    onCleanup(() => stop?.());

    return {
        isCopied,
        async copy(text) {
            // write the text, then show it as copied until the duration passes
            await writeClipboard(text);
            stop?.();
            setIsCopied(true);
            stop = makeTimer(() => setIsCopied(false), COPIED_DURATION, setTimeout);
        },
    };
}

/** The properties of a copy button, a button's properties included. */
export interface CopyButtonProperties extends Omit<ButtonProperties, "onClick"> {
    /** The text the button copies. */
    readonly value: string;
    /** Handle the browser refusing the copy; without it the refusal propagates. */
    readonly onFailure?: (error: unknown) => void;
}

/** Render a button that copies a text, shows a check mark for a moment and announces the copy, labelled by its children or named Copy. */
export function CopyButton(properties: CopyButtonProperties): JSX.Element {
    // read the labels and keep a clipboard for the button
    const button = merge(DEFAULTS, properties);
    const rest = omit(button, "value", "onFailure", "children");
    const copied = createCopy();

    // copy, handing a refusal to the caller or letting it propagate
    const click = (): void => {
        copied.copy(properties.value).catch((error: unknown) => {
            if (properties.onFailure === undefined) {
                throw error;
            }
            properties.onFailure(error);
        });
    };

    return (
        <Button
            data-slot="copy-button"
            data-state={copied.isCopied() ? "copied" : "idle"}
            aria-label={properties.children === undefined ? "Copy" : undefined}
            {...rest}
            onClick={click}
        >
            <Icon icon={copied.isCopied() ? check : copy} />
            {properties.children}
            <span role="status" {...style.attrs(visuallyHiddenStyle())}>
                <Show when={copied.isCopied()}>{"Copied"}</Show>
            </span>
        </Button>
    );
}
