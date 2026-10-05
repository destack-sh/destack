import * as stylex from "@destack/style";
import { For, type JSX } from "@destack/view";

/** Fade a part of a figure in anew each time the stack switches. */
export function Fade(properties: {
    isOpen: boolean;
    style?: stylex.Styles;
    component?: string;
    children: JSX.Element;
}) {
    // key the part by the state so every switch builds it anew
    return (
        <For each={[properties.isOpen]}>
            {() => (
                <div
                    data-fade
                    data-component={properties.component}
                    {...stylex.attrs(properties.style)}
                >
                    {properties.children}
                </div>
            )}
        </For>
    );
}
