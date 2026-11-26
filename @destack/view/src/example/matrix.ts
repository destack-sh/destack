import type { BrowserEnvironment } from "@destack/environment/browser";
import type { JsonObject } from "@destack/schema";
import type { MatrixAxes } from "../page/message.ts";
import { type ExampleFrame, frameStyle, renderExample } from "./frame.ts";

/** Lay an example out once per cell of its axes, telling the cell a person picks or opens. */
export function renderMatrix(
    element: HTMLElement,
    frame: ExampleFrame & { readonly properties: JsonObject },
    axes: MatrixAxes,
    environment: BrowserEnvironment,
    on: {
        /** Handle a person picking a cell: the values it lays out. */
        readonly choose: (values: JsonObject) => void;
        /** Handle a person opening a cell on its own. */
        readonly open: (values: JsonObject) => void;
    },
): () => void {
    // paint the grid as an example's frame, its first column the rows' names
    const grid = document.createElement("div");
    const { columns, rows } = axes;
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
    element.append(grid);

    // name each column, and each row before its cells
    const disposals: (() => void)[] = [];
    grid.append(caption(""));
    for (const { title } of columns.values) {
        grid.append(caption(title));
    }
    for (const row of rows?.values ?? [undefined]) {
        grid.append(caption(row?.title ?? ""));

        // render the example in each cell with the cell's values, picked by a press on it
        for (const column of columns.values) {
            const values: JsonObject = {
                ...frame.properties,
                [columns.name]: column.value,
                ...(rows === undefined || row === undefined ? {} : { [rows.name]: row.value }),
            };
            // ring the cell whose values are the example's own
            const isChosen =
                JSON.stringify(frame.properties[columns.name]) === JSON.stringify(column.value) &&
                (rows === undefined ||
                    row === undefined ||
                    JSON.stringify(frame.properties[rows.name]) === JSON.stringify(row.value));
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
            const body = document.createElement("div");
            body.style.pointerEvents = "none";
            cell.append(body);
            disposals.push(
                renderExample(
                    body,
                    {
                        ...frame,
                        example: { ...frame.example, layout: "padded" },
                        properties: values,
                    },
                    environment,
                ),
            );
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
            pick.addEventListener("click", () => on.choose(values));
            pick.addEventListener("dblclick", () => on.open(values));
            cell.append(pick);
            grid.append(cell);
        }
    }

    return () => {
        for (const dispose of disposals) {
            dispose();
        }
        grid.remove();
    };
}

/** Create the caption of a column or a row. */
function caption(text: string): HTMLElement {
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
