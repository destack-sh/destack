import * as style from "@destack/style";
import { color, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createEffect, createUniqueId, For, type JSX, omit, onSettled, Show } from "@destack/view";
import type { ChartPoint, ChartTheme, ChartValue, DomChartDefinition } from "@tanstack/charts";
import { createChartRendererAdapter } from "@tanstack/charts/adapter/renderer";
import { createCanvasChartRenderer } from "@tanstack/charts/canvas";
import { renderChartSvg } from "@tanstack/charts/svg";
import { createSvgChartRenderer } from "@tanstack/charts/svg/renderer";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The height of a chart that sets neither a height nor an aspect ratio, as the upstream host defaults it. */
const DEFAULT_HEIGHT = 320;

/** The scene colors every chart takes from the theme's roles: series from the accent, guides from the foreground the renderer attenuates. */
const THEME: ChartTheme = {
    foreground: color.foreground,
    muted: color.foreground,
    grid: color.foreground,
    background: "transparent",
    palette: [color.chart1, color.chart2, color.chart3, color.chart4, color.chart5],
};

/** The styles of a chart, its surface, its tooltip and its table. */
const styles = style.create({
    chart: {
        position: "relative",
        width: "100%",
        color: color.foreground,
        "--ts-chart-tooltip-background": color.popover,
        "--ts-chart-tooltip-color": color.popoverForeground,
        "--ts-chart-tooltip-border": `${stroke.border} solid ${color.border}`,
        "--ts-chart-tooltip-border-radius": radius[3],
        "--ts-chart-tooltip-shadow": shadow.overlay,
        "--ts-chart-tooltip-padding": `${space[1]} ${space[2]}`,
        "--ts-chart-tooltip-font": "inherit",
        "--ts-chart-focus-fill": color.background,
    },
    tall: (height: string) => ({ height }),
    proportioned: (ratio: string) => ({ aspectRatio: ratio }),
    surface: {
        width: "100%",
        height: "100%",
    },
});

/** A column of a chart's table: the field of each row it shows and its heading. */
export interface ChartColumn<Datum> {
    /** The field of each row. */
    readonly key: keyof Datum & string;
    /** The heading. */
    readonly label: string;
    /** Write a row's value as text, String by default. */
    readonly format?: (value: Datum[keyof Datum & string]) => string;
}

/** The table a chart reads out to assistive technology: its rows and columns. */
export interface ChartTable<Datum> {
    /** The rows, such as the data the marks draw. */
    readonly rows: readonly Datum[];
    /** The columns, in order. */
    readonly columns: readonly ChartColumn<Datum>[];
}

/** The properties of a chart, the native element's attributes included. */
export interface ChartProperties<
    Datum,
    XValue extends ChartValue = ChartValue,
    YValue extends ChartValue = ChartValue,
    Row = Datum,
> extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "onSelect"> {
    /** The marks, scales and guides, from `defineChart`. */
    readonly definition: DomChartDefinition<Datum, XValue, YValue>;
    /** The accessible name, such as "Weekly downloads by package". */
    readonly label: string;
    /** The context a reader needs beyond the name, such as units or gaps. */
    readonly description?: string;
    /** What draws the scene: SVG, readable before the script runs, or canvas for large datasets, SVG by default. */
    readonly renderer?: "svg" | "canvas";
    /** The height in pixels, 320 unless an aspect ratio sets it. */
    readonly height?: number;
    /** The width over the height, which sets the height from the measured width. */
    readonly aspectRatio?: number;
    /** The exact values read out beside the drawing, a visually hidden table of the source rows. */
    readonly table?: ChartTable<Row>;
    /** Handle the focused point changing, null once none is focused. */
    readonly onFocusChange?: (point: ChartPoint<Datum, XValue, YValue> | null) => void;
    /** Handle a point being chosen with a click or Enter, null once cleared. */
    readonly onSelect?: (point: ChartPoint<Datum, XValue, YValue> | null) => void;
    /** The StyleX styles applied after the chart's styles. */
    readonly xstyle?: style.Styles;
}

/** Draw a chart as an SVG in the theme's colors, its points reachable with the keyboard and its values in an optional table. */
export function Chart<
    Datum,
    XValue extends ChartValue = ChartValue,
    YValue extends ChartValue = ChartValue,
    Row = Datum,
>(properties: ChartProperties<Datum, XValue, YValue, Row>): JSX.Element {
    // render the scene on the server and in the browser before it mounts, under one id prefix
    const rest = omit(
        properties,
        "definition",
        "label",
        "description",
        "renderer",
        "height",
        "aspectRatio",
        "table",
        "onFocusChange",
        "onSelect",
        "xstyle",
        "style",
    );
    const prefix = `chart-${createUniqueId().replaceAll(/[^\w-]/gu, "")}`;
    const renderer =
        properties.renderer === "canvas"
            ? createCanvasChartRenderer<Datum, XValue, YValue>()
            : createSvgChartRenderer<Datum, XValue, YValue>(renderChartSvg);
    const options = () => ({
        definition: themed(properties.definition),
        renderer,
        ariaLabel: properties.label,
        idPrefix: prefix,
        ...(properties.description === undefined
            ? {}
            : { ariaDescription: properties.description }),
        ...(properties.height === undefined ? {} : { height: properties.height }),
        ...(properties.aspectRatio === undefined ? {} : { aspectRatio: properties.aspectRatio }),
        onFocusChange: (point: ChartPoint<Datum, XValue, YValue> | null) =>
            properties.onFocusChange?.(point),
        onSelect: (point: ChartPoint<Datum, XValue, YValue> | null) => properties.onSelect?.(point),
    });
    const adapter = createChartRendererAdapter<Datum, XValue, YValue>(options());
    const markup = adapter.prerender();

    // mount the host over the rendered markup, following changed properties until unmounted
    let surface: HTMLDivElement | undefined;
    createEffect(options, (next) => adapter.update(next));
    onSettled(() => {
        if (surface !== undefined) {
            adapter.mount(surface);
        }

        return () => adapter.destroy();
    });

    // size the host as the upstream host does until it measures itself
    const size = (): style.Styles =>
        properties.height === undefined && properties.aspectRatio !== undefined
            ? styles.proportioned(String(properties.aspectRatio))
            : styles.tall(`${String(properties.height ?? DEFAULT_HEIGHT)}px`);

    return (
        <div
            data-slot="chart"
            {...rest}
            {...style.attributes(
                [text.footnote, styles.chart, size(), properties.xstyle],
                properties.style,
            )}
        >
            {/* Drawing */}
            <div
                data-slot="chart-surface"
                ref={(element) => {
                    surface = element;
                }}
                innerHTML={markup}
                {...style.attrs(styles.surface)}
            />

            {/* Values */}
            <Show when={properties.table}>
                {(table) => (
                    <table data-slot="chart-table" {...style.attrs(visuallyHiddenStyle())}>
                        <caption>{properties.label}</caption>
                        <thead>
                            <tr>
                                <For each={table().columns}>
                                    {(column) => <th scope="col">{column.label}</th>}
                                </For>
                            </tr>
                        </thead>
                        <tbody>
                            <For each={table().rows}>
                                {(row) => (
                                    <tr>
                                        <For each={table().columns}>
                                            {(column) => <td>{cellOf(row, column)}</td>}
                                        </For>
                                    </tr>
                                )}
                            </For>
                        </tbody>
                    </table>
                )}
            </Show>
        </div>
    );
}

/** Give a definition the theme's scene colors under any it sets itself. */
function themed<Datum, XValue extends ChartValue, YValue extends ChartValue>(
    definition: DomChartDefinition<Datum, XValue, YValue>,
): DomChartDefinition<Datum, XValue, YValue> {
    // theme a responsive definition's every build
    if ("chart" in definition) {
        const build = definition.chart;

        return {
            ...definition,
            chart: (context) => {
                const spec = build(context);

                return { ...spec, theme: { ...THEME, ...spec.theme } };
            },
        };
    }

    return { ...definition, theme: { ...THEME, ...definition.theme } };
}

/** Write one cell of a chart's table. */
function cellOf<Datum>(row: Datum, column: ChartColumn<Datum>): string {
    const value = row[column.key];

    return column.format === undefined ? String(value) : column.format(value);
}
