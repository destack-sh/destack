import * as style from "@destack/style";
import { For, type JSX } from "@destack/view";

/**
 * Build a part of a figure anew each time the stack switches, and bring it in.
 *
 * A small part flips in, and a large part fades in through a slight blur so it never swings.
 * A part given a turn also fades in through the blur each time the turn changes, stacked or not.
 */
export function Fade(properties: {
    isOpen: boolean;
    xstyle?: style.Styles;
    component?: string;
    isLarge?: boolean;
    turn?: string | number;
    children: JSX.Element;
}) {
    // key the part by the state so every switch builds it anew
    return (
        <For each={[`${String(properties.isOpen)}:${String(properties.turn ?? "")}`]}>
            {() => (
                <div
                    data-fade={properties.isLarge === true ? "large" : "small"}
                    data-turn={properties.turn === undefined ? undefined : ""}
                    data-component={properties.component}
                    {...style.attrs(properties.xstyle)}
                >
                    {properties.children}
                </div>
            )}
        </For>
    );
}
