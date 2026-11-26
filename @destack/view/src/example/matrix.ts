import type { BrowserEnvironment } from "@destack/environment/browser";
import type { JsonObject } from "@destack/schema";
import type { MatrixAxes } from "../page/message.ts";
import { type ExampleFrame, frameStyle, renderExample } from "./frame.ts";

/** The smallest side of a framed cell's page in CSS pixels, 25rem. */
const FRAME_MINIMUM = 400;

/** The room around an overlay in its frame, in CSS pixels. */
const FRAME_MARGIN = 16;

/** The window a framed fullscreen example fills, a small laptop's in CSS pixels. */
const WINDOW = { width: 1024, height: 640 };

/** An open overlay in the top layer, which covers the page instead of its cell. */
const OVERLAY = ":modal, :popover-open";

/** How a matrix draws its cells: in its own page, or each in a frame of the example's page. */
type MatrixLayout =
    | { readonly kind: "page" }
    | { readonly kind: "frame"; readonly width: number; readonly height: number };

/** What a matrix tells about its cells: the one a person picks, and the one they open. */
export interface MatrixHandlers {
    /** Handle a person picking a cell: the values it lays out. */
    readonly choose: (values: JsonObject) => void;
    /** Handle a person opening a cell on its own: the values it lays out. */
    readonly open: (values: JsonObject) => void;
}

/** An example laid out once per cell of its axes, which a person picks a cell of. */
export class Matrix {
    /** The element the matrix draws in. */
    readonly element: HTMLElement;
    /** The name of the example in its page's library. */
    readonly name: string;
    /** The example and how each cell frames it. */
    readonly #frame: ExampleFrame;
    /** The environment each cell renders in. */
    readonly #environment: BrowserEnvironment;
    /** What the matrix tells about its cells. */
    readonly #on: MatrixHandlers;
    /** The example's values, which every cell starts from. */
    #values: JsonObject;
    /** The axes the cells lie along. */
    #axes: MatrixAxes;
    /** The drawn grid, absent once disposed. */
    #grid: HTMLElement | undefined;
    /** The unmounting of each cell's example. */
    #disposals: (() => void)[];
    /** How the cells draw, in frames once an example opens an overlay. */
    #layout: MatrixLayout;

    /** Draw an example's matrix in an element, from its values along its axes. */
    constructor(
        element: HTMLElement,
        name: string,
        frame: ExampleFrame,
        values: JsonObject,
        axes: MatrixAxes,
        environment: BrowserEnvironment,
        on: MatrixHandlers,
    ) {
        // store the frame, the values and the axes, then draw
        this.element = element;
        this.name = name;
        this.#frame = frame;
        this.#environment = environment;
        this.#on = on;
        this.#values = values;
        this.#axes = axes;
        this.#grid = undefined;
        this.#disposals = [];
        this.#layout =
            frame.example.layout === "fullscreen" ? { kind: "frame", ...WINDOW } : { kind: "page" };
        this.#draw();
    }

    /** Change some of the example's values, drawing every cell again. */
    set(values: JsonObject): void {
        this.#values = { ...this.#values, ...values };
        this.dispose();
        this.#draw();
    }

    /** Lay the cells out along other axes, drawing them again. */
    arrange(axes: MatrixAxes): void {
        this.#axes = axes;
        this.dispose();
        this.#draw();
    }

    /** Take every cell down and remove the grid. */
    dispose(): void {
        // unmount each cell, then drop the grid
        for (const dispose of this.#disposals) {
            dispose();
        }
        this.#disposals = [];
        this.#grid?.remove();
        this.#grid = undefined;
    }

    /** Draw the grid: a caption per column and row, and the example in each cell. */
    #draw(): void {
        // paint the grid as an example's frame, its first column the rows' names
        const document = this.element.ownerDocument;
        const grid = document.createElement("div");
        const { columns, rows } = this.#axes;
        const frame = this.#frame;
        const properties = this.#values;
        for (const [name, value] of Object.entries(frameStyle(frame.globals ?? {}, frame.theme))) {
            grid.style.setProperty(name, value);
        }
        Object.assign(grid.style, {
            display: "grid",
            gridTemplateColumns: `auto repeat(${String(columns.values.length)}, minmax(6rem, 1fr))`,
            gridAutoRows: "min-content",
            alignContent: "start",
            alignItems: "stretch",
            gap: "var(--destack-space-3)",
            padding: "var(--destack-space-4) var(--destack-space-6) var(--destack-space-6)",
            boxSizing: "border-box",
            minHeight: "100dvh",
            background: "var(--destack-color-muted)",
        });
        this.element.append(grid);
        this.#grid = grid;

        // name each column, and each row before its cells
        grid.append(caption(document, ""));
        for (const { title } of columns.values) {
            grid.append(caption(document, title));
        }
        for (const row of rows?.values ?? [undefined]) {
            grid.append(caption(document, row?.title ?? ""));

            // render the example in each cell with the cell's values, picked by a press on it
            for (const column of columns.values) {
                const values: JsonObject = {
                    ...properties,
                    [columns.name]: column.value,
                    ...(rows === undefined || row === undefined ? {} : { [rows.name]: row.value }),
                };
                // ring the cell whose values are the example's own
                const isChosen =
                    JSON.stringify(properties[columns.name]) === JSON.stringify(column.value) &&
                    (rows === undefined ||
                        row === undefined ||
                        JSON.stringify(properties[rows.name]) === JSON.stringify(row.value));
                const cell = document.createElement("div");
                Object.assign(cell.style, {
                    position: "relative",
                    display: "grid",
                    placeItems: "center",
                    minHeight: "6rem",
                    borderRadius: "var(--destack-radius-3)",
                    border: "var(--destack-stroke-border) solid var(--destack-color-border)",
                    background: "var(--destack-color-background)",
                    overflow: "hidden",
                    ...(isChosen
                        ? {
                              borderColor: "var(--destack-color-ring)",
                              boxShadow: "0 0 0 1px var(--destack-color-ring)",
                          }
                        : {}),
                });

                // keep the cell's example look-only, out of the focus and the pointer
                const body = document.createElement("div");
                body.inert = true;
                cell.append(body);
                grid.append(cell);

                // frame the example's page with the cell's values, scaled to the cell's width
                const layout = this.#layout;
                if (layout.kind === "frame") {
                    body.append(this.#page(values, layout));
                    Object.assign(body.style, { alignSelf: "stretch", justifySelf: "stretch" });
                    Object.assign(cell.style, {
                        containerType: "inline-size",
                        aspectRatio: `${String(layout.width)} / ${String(layout.height)}`,
                    });
                }
                // render the example in the page once the cell is in it, as a dialog opens only there
                else {
                    this.#disposals.push(
                        renderExample(
                            body,
                            {
                                ...frame,
                                example: { ...frame.example, layout: "padded" },
                                properties: values,
                            },
                            this.#environment,
                        ),
                    );
                }

                // draw every cell in a frame sized to the overlay once one opens over the page
                const overlay = body.querySelector(OVERLAY);
                if (overlay !== null) {
                    this.#layout = frameOf(body, overlay);
                    this.dispose();
                    this.#draw();

                    return;
                }

                // pick the cell on a press, open it on a double press
                const pick = document.createElement("button");
                pick.type = "button";
                pick.ariaPressed = isChosen ? "true" : "false";
                pick.ariaLabel = row === undefined ? column.title : `${column.title}, ${row.title}`;
                Object.assign(pick.style, {
                    position: "absolute",
                    inset: "0",
                    border: "0",
                    background: "transparent",
                    cursor: "pointer",
                });
                pick.addEventListener("click", () => this.#on.choose(values));
                pick.addEventListener("dblclick", () => this.#on.open(values));
                cell.append(pick);
            }
        }
    }

    /** Create a frame of the example's page alone, with a cell's values. */
    #page(
        values: JsonObject,
        layout: Extract<MatrixLayout, { readonly kind: "frame" }>,
    ): HTMLIFrameElement {
        // address the example's page alone and look-only, with its globals and the cell's values
        const url = new URL(this.#environment.navigation.currentEntry.url);
        url.search = new URLSearchParams({
            name: this.name,
            globals: JSON.stringify(this.#frame.globals ?? {}),
            properties: JSON.stringify(values),
            inert: "",
        }).toString();

        // fill the cell with it
        const page = this.element.ownerDocument.createElement("iframe");
        page.src = url.href;
        page.title = this.name;
        const width = `${String(layout.width)}px`;
        Object.assign(page.style, {
            display: "block",
            width,
            height: `${String(layout.height)}px`,
            border: "0",
            transformOrigin: "0 0",
            scale: `calc(100cqi / ${width})`,
        });

        return page;
    }
}

/** Size an example's frames by its open overlay: a modal alone, an anchored one on any side. */
function frameOf(body: HTMLElement, overlay: Element): MatrixLayout {
    // measure what opens the overlay, and the overlay
    const anchor = body.getBoundingClientRect();
    const open = overlay.getBoundingClientRect();
    const isModal = overlay.matches(":modal");

    // leave a modal room for itself, an anchored one room on each side of its anchor
    const width = isModal
        ? open.width + 2 * FRAME_MARGIN
        : anchor.width + 2 * (open.width + FRAME_MARGIN);
    const height = isModal
        ? open.height + 2 * FRAME_MARGIN
        : anchor.height + 2 * (open.height + FRAME_MARGIN);

    return {
        kind: "frame",
        width: Math.ceil(Math.max(width, FRAME_MINIMUM)),
        height: Math.ceil(Math.max(height, FRAME_MINIMUM)),
    };
}

/** Create the caption of a column or a row. */
function caption(document: Document, text: string): HTMLElement {
    // write the caption in the theme's small muted text
    const element = document.createElement("span");
    element.textContent = text;
    Object.assign(element.style, {
        alignSelf: "center",
        font: "var(--destack-text-xs-font-size) / var(--destack-text-xs-line-height) var(--destack-font-text)",
        color: "var(--destack-color-muted-foreground)",
    });

    return element;
}
