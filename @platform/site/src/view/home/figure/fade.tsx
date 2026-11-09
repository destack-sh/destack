import * as style from "@destack/style";
import { For, type JSX, onSettled } from "@destack/view";

/** Build a figure's part anew on each switch, bringing it in at once while off screen. */
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
            {() => {
                // hold the part still when its figure is off screen as it is built
                let part: HTMLDivElement | undefined;
                onSettled(() => {
                    part?.toggleAttribute("data-still", part.closest("[data-asleep]") !== null);
                });

                return (
                    <div
                        ref={part}
                        data-fade={properties.isLarge === true ? "large" : "small"}
                        data-turn={properties.turn === undefined ? undefined : ""}
                        data-component={properties.component}
                        {...style.attrs(properties.xstyle)}
                    >
                        {properties.children}
                    </div>
                );
            }}
        </For>
    );
}
