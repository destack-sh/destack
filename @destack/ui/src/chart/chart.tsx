import { renderPart, type Render } from "../part/index.ts";
import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { Reference } from "@destack/style";
import { text } from "@destack/theme/text";
import {
    createEffect,
    createUniqueId,
    For,
    type JSX,
    omit,
    onSettled,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import type { ChartPoint, ChartTheme, ChartValue, DomChartDefinition } from "@tanstack/charts";
import { createChartRendererAdapter } from "@tanstack/charts/adapter/renderer";
import { createCanvasChartRenderer } from "@tanstack/charts/canvas";
import { renderChartSvg } from "@tanstack/charts/svg";
import { createSvgChartRenderer } from "@tanstack/charts/svg/renderer";
import { Length } from "../virtualizer/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The height of a chart that sets neither a height nor an aspect ratio. */
const DEFAULT_HEIGHT = 320;

/** The size of the text the scene grows with. */
const TEXT_SIZE = Reference.css("{text.xs.fontSize}");

/** That size at the default text size, in pixels, which the engine's own sizes suit. */
const BASE_TEXT = 12;

/** The scene colors every chart takes from the theme's roles. */
const THEME: ChartTheme = {
    foreground: color.foreground,
    muted: color.foreground,
    grid: color.foreground,
    background: "transparent",
    palette: [
        swatch.series1Mark,
        swatch.series2Mark,
        swatch.series3Mark,
        swatch.series4Mark,
        swatch.series5Mark,
    ],
};

/** The styles of a chart, its surface, its tooltip and its table. */
const styles = style.create({
    chart: {
        position: "relative",
        width: "100%",
        color: color.foreground,
        "--ts-chart-tooltip-background": color.popover,
        "--ts-chart-tooltip-color": color.foreground,
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
    // lay it out smaller, and grow it back with its text and its tooltip
    grown: (length: string, transform: string, transformOrigin: string) => ({
        width: length,
        height: length,
        transform,
        transformOrigin,
    }),
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
    /** What draws the scene, SVG by default. */
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
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Draw a chart as an SVG in the theme's colors. */
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
        "children",
    );
    const prefix = `chart-${createUniqueId().replaceAll(/[^\w-]/gu, "")}`;
    const renderer =
        untrack(() => properties.renderer) === "canvas"
            ? createCanvasChartRenderer<Datum, XValue, YValue>()
            : createSvgChartRenderer<Datum, XValue, YValue>(renderChartSvg);

    // lay the scene out as at the default text size, and grow it with the text
    const locale = useLocale();
    const textLength = new Length(() => TEXT_SIZE);
    const scale = (): number => (textLength.pixels() ?? BASE_TEXT) / BASE_TEXT;
    const isProportioned = (): boolean =>
        properties.height === undefined && properties.aspectRatio !== undefined;
    const height = (): number => (properties.height ?? DEFAULT_HEIGHT) / scale();
    const options = () => ({
        definition: themed(properties.definition),
        renderer,
        ariaLabel: properties.label,
        idPrefix: prefix,
        ...(properties.description === undefined
            ? {}
            : { ariaDescription: properties.description }),
        ...(isProportioned() || (properties.height === undefined && scale() === 1)
            ? {}
            : { height: height() }),
        ...(properties.aspectRatio === undefined ? {} : { aspectRatio: properties.aspectRatio }),
        onFocusChange: (point: ChartPoint<Datum, XValue, YValue> | null) =>
            properties.onFocusChange?.(point),
        onSelect: (point: ChartPoint<Datum, XValue, YValue> | null) => properties.onSelect?.(point),
    });
    const adapter = createChartRendererAdapter<Datum, XValue, YValue>(untrack(options));
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
        isProportioned()
            ? styles.proportioned(String(properties.aspectRatio))
            : styles.tall(`${String(properties.height ?? DEFAULT_HEIGHT)}px`);

    // fill the host once grown
    const grown = (): style.Styles => {
        // leave it as the engine lays it out at the default text size
        const factor = scale();

        return factor === 1
            ? null
            : styles.grown(
                  `${String(100 / factor)}%`,
                  `scale(${String(factor)})`,
                  locale.direction === "rtl" ? "100% 0" : "0 0",
              );
    };

    return renderPart("div", "chart", rest, () => [text.xs, styles.chart, size()], {
        get children() {
            return (
                <>
                    {/* Drawing */}
                    <div
                        data-slot="chart-surface"
                        ref={(element) => {
                            surface = element;
                        }}
                        innerHTML={markup}
                        {...style.attributes([styles.surface, grown()])}
                    />

                    {textLength.render()}

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
                </>
            );
        },
    });
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
