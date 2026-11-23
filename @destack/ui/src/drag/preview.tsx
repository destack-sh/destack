import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color, shadow } from "@destack/theme/tokens.stylex";
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
    raised: {
        backgroundColor: color.background,
        boxShadow: shadow.overlay,
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
            : operation.draggable;
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

                // set a table row's cells at their widths inside a table around it
                const shown =
                    element instanceof HTMLTableRowElement ? previewOf(element, copy) : copy;
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
                            created.append(shown);
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

/** Raise a row's copy above the page in a table like the row's, each cell as wide as before. */
function previewOf(row: HTMLTableRowElement, copy: HTMLElement): HTMLElement {
    // size each copied cell as its source cell stands, the copy's cells matching the row's
    const cells = [...row.cells];
    for (const [index, cell] of [...copy.children].entries()) {
        const width = present(
            cells[index],
            `the dragged row's cell ${String(index)}`,
        ).getBoundingClientRect().width;
        if (cell instanceof HTMLElement) {
            cell.style.width = `${String(width)}px`;
            cell.style.minWidth = `${String(width)}px`;
            cell.style.maxWidth = `${String(width)}px`;
        }
    }

    // put the row in a table and body of the row's width, the table styled as the row's table
    const source = present(row.closest("table"), "the dragged row's table");
    const table = row.ownerDocument.createElement("table");
    const body = row.ownerDocument.createElement("tbody");
    table.className = source.className;
    table.style.width = `${String(row.getBoundingClientRect().width)}px`;
    body.append(copy);
    table.append(body);

    // raise the table above the page on the preview's background and shadow
    const preview = row.ownerDocument.createElement("div");
    preview.className = present(style.attrs(styles.raised).class, "the raised row's class");
    preview.append(table);

    return preview;
}
