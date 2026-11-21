import * as style from "@destack/style";
import { createMemo, type JSX, onSettled, Show } from "@destack/view";
import type { DragManager } from "./drag.ts";

/** The styles of a drag's preview. */
const styles = style.create({
    preview: {
        position: "fixed",
        top: 0,
        right: "auto",
        bottom: "auto",
        left: 0,
        margin: 0,
        padding: 0,
        borderStyle: "none",
        overflow: "visible",
        color: "inherit",
        backgroundColor: "transparent",
        pointerEvents: "none",
    },
    place: (x: number, y: number, width: number, height: number) => ({
        width: `${String(width)}px`,
        height: `${String(height)}px`,
        translate: `${String(x)}px ${String(y)}px`,
    }),
});

/** The properties of a drag's preview. */
export interface DragPreviewProperties {
    /** The drags. */
    readonly manager: DragManager;
    /** Read the element a dragged item shows as. */
    readonly element: (id: string) => HTMLElement | undefined;
}

/** Render a copy of the item a pointer drags above the page. */
export function DragPreview(properties: DragPreviewProperties): JSX.Element {
    // preview a pointer's drag only
    const source = createMemo(() => {
        const operation = properties.manager.operation();

        return operation === undefined || operation.pointer === "keyboard"
            ? undefined
            : operation.source;
    });

    return (
        <Show when={source()} keyed>
            {(id) => {
                // copy the item where it stands as the drag lifts
                const element = properties.element(id);
                const bounds = element?.getBoundingClientRect();
                const copy = element?.cloneNode(true);
                if (bounds === undefined || !(copy instanceof HTMLElement)) {
                    return null;
                }

                // show the copy at full strength under no id of the item's
                copy.style.opacity = "1";
                for (const each of [copy, ...copy.querySelectorAll("[id]")]) {
                    each.removeAttribute("id");
                }
                const operation = () => properties.manager.operation();

                // raise the copy above the page once it stands in the document
                let preview: HTMLDivElement | undefined;
                onSettled(() => preview?.showPopover());

                return (
                    <div
                        popover="manual"
                        inert
                        aria-hidden="true"
                        data-slot="drag-preview"
                        ref={(created) => {
                            preview = created;
                            created.append(copy);
                        }}
                        {...style.attrs(
                            styles.preview,
                            styles.place(
                                bounds.left + (operation()?.offsetX ?? 0),
                                bounds.top + (operation()?.offsetY ?? 0),
                                bounds.width,
                                bounds.height,
                            ),
                        )}
                    />
                );
            }}
        </Show>
    );
}
