import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, swatch, weight, width } from "@destack/theme/tokens.stylex";
import { Badge } from "../badge/index.ts";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Chart } from "./chart.tsx";
import { areaY, barY, colorLegend, defineChart, dot, lineY } from "./index.ts";
import { pie, polar, radialArc } from "./module/polar.ts";
import { scaleBand } from "./module/scales/band.ts";
import { scaleLinear } from "./module/scales/linear.ts";
import { scalePoint } from "./module/scales/point.ts";
import { tooltip } from "./module/tooltip.ts";

/** The notes written each month in two notebooks. */
const WRITTEN = [
    { month: "Jan", notebook: "Work", notes: 18 },
    { month: "Feb", notebook: "Work", notes: 24 },
    { month: "Mar", notebook: "Work", notes: 21 },
    { month: "Apr", notebook: "Work", notes: 30 },
    { month: "Jan", notebook: "Trips", notes: 6 },
    { month: "Feb", notebook: "Trips", notes: 4 },
    { month: "Mar", notebook: "Trips", notes: 11 },
    { month: "Apr", notebook: "Trips", notes: 15 },
] as const;

/** The notes in each notebook. */
const NOTEBOOKS = [
    { notebook: "Work", notes: 93 },
    { notebook: "Trips", notes: 36 },
    { notebook: "Recipes", notes: 22 },
    { notebook: "Reading", notes: 14 },
] as const;

/** The length and the edits of each note. */
const NOTES = [
    { title: "Plan", words: 120, edits: 4 },
    { title: "Lisbon", words: 860, edits: 12 },
    { title: "Pasta", words: 310, edits: 3 },
    { title: "Budget", words: 540, edits: 19 },
    { title: "Reading list", words: 220, edits: 8 },
] as const;

/** The notes written each month in the work notebook. */
const WORK = WRITTEN.filter((row) => row.notebook === "Work");

/** The accent fading from a third to nothing down an area, as a chart's fill. */
const FADE = {
    id: "fade",
    x1: 0,
    y1: 0,
    x2: 0,
    y2: 1,
    stops: [
        { offset: 0, color: swatch.series1Mark, opacity: 0.5 },
        { offset: 1, color: swatch.series1Mark, opacity: 0 },
    ],
} as const;

/** The months on the horizontal axis, spaced evenly. */
const MONTHS = { scale: () => scalePoint().padding(0.4) };

/** Ten thousand response times, one a second, a deterministic ripple over a slow rise. */
const READINGS = Array.from({ length: 10_000 }, (_, at) => ({
    at,
    milliseconds: 120 + at / 100 + 40 * Math.sin(at / 37) * Math.cos(at / 11),
}));

/** A line per notebook of the notes written each month, with a tooltip and a legend. */
export const chartLine = defineExample({
    of: Chart,
    name: "line",
    description: t`A line per notebook of the notes written each month, with a tooltip and a legend.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Notes written each month by notebook"
            definition={defineChart({
                marks: [lineY(WRITTEN, { x: "month", y: "notes", z: "notebook" })],
                scales: {
                    x: MONTHS,
                    y: { scale: scaleLinear, nice: true, grid: true },
                },
                color: { legend: colorLegend() },
                tooltip,
            })}
            table={{
                rows: WRITTEN,
                columns: [
                    { key: "month", label: "Month" },
                    { key: "notebook", label: "Notebook" },
                    { key: "notes", label: "Notes" },
                ],
            }}
        />
    ),
});

/** Ten thousand readings drawn as dots on a canvas, as large datasets draw. */
export const chartCanvas = defineExample({
    of: Chart,
    name: "canvas",
    description: t`Ten thousand readings drawn as dots on a canvas, as large datasets draw.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Ten thousand response times"
            renderer="canvas"
            definition={defineChart({
                marks: [dot(READINGS, { x: "at", y: "milliseconds" })],
                scales: {
                    x: { scale: scaleLinear },
                    y: { scale: scaleLinear, nice: true, grid: true },
                },
            })}
        />
    ),
});

/** The notes written each month in the work notebook as a filled area. */
export const chartArea = defineExample({
    of: Chart,
    name: "area",
    description: t`The notes written each month in the work notebook as a filled area.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Notes written each month in Work"
            height={240}
            definition={defineChart({
                marks: [
                    areaY(WORK, { x: "month", y: "notes", fill: `url(#${FADE.id})` }),
                    lineY(WORK, { x: "month", y: "notes", strokeWidth: 2 }),
                ],
                scales: {
                    x: MONTHS,
                    y: { scale: scaleLinear, nice: true, grid: true },
                },
                gradients: [FADE],
                tooltip,
            })}
        />
    ),
});

/** A bar per notebook of the notes in it. */
export const chartBar = defineExample({
    of: Chart,
    name: "bar",
    description: t`A bar per notebook of the notes in it.`,
    layout: "padded",
    properties: {
        height: 240,
    },
    render: (properties) => (
        <Chart
            {...properties}
            xstyle={card.wide}
            label="Notes per notebook"
            definition={defineChart({
                marks: [barY(NOTEBOOKS, { x: "notebook", y: "notes", radius: { end: 4 } })],
                scales: {
                    x: { scale: () => scaleBand().padding(0.4), axis: { label: "Notebook" } },
                    y: { scale: scaleLinear, nice: true, grid: true, axis: { label: "Notes" } },
                },
                tooltip,
            })}
        />
    ),
});

/** A dot per note of its length against its edits. */
export const chartScatter = defineExample({
    of: Chart,
    name: "scatter",
    description: t`A dot per note of its length against its edits.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Length and edits of each note"
            description="Words on the horizontal axis, edits on the vertical axis."
            definition={defineChart({
                marks: [dot(NOTES, { x: "words", y: "edits" })],
                scales: {
                    x: { scale: scaleLinear, nice: true, axis: { label: "Words" } },
                    y: { scale: scaleLinear, nice: true, grid: true, axis: { label: "Edits" } },
                },
                tooltip,
            })}
        />
    ),
});

/** A donut of the share of notes in each notebook. */
export const chartPie = defineExample({
    of: Chart,
    name: "pie",
    description: t`A donut of the share of notes in each notebook.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Share of notes per notebook"
            height={240}
            definition={defineChart({
                marks: [
                    polar({
                        inset: 8,
                        marks: [
                            radialArc(pie(NOTEBOOKS, { value: "notes" }), {
                                innerRadius: ({ radius }) => radius * 0.6,
                                color: "notebook",
                                key: "notebook",
                            }),
                        ],
                        scales: { angle: null, radius: null },
                    }),
                ],
                scales: { x: null, y: null },
                color: { legend: colorLegend() },
                tooltip,
            })}
            table={{
                rows: NOTEBOOKS,
                columns: [
                    { key: "notebook", label: "Notebook" },
                    { key: "notes", label: "Notes" },
                ],
            }}
        />
    ),
});

/** The styles of the chart card: its width and its headline figure. */
const card = style.create({
    wide: {
        maxWidth: "40rem",
    },
    donut: {
        position: "relative",
        width: "20rem",
    },
    middle: {
        position: "absolute",
        inset: 0,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        pointerEvents: "none",
    },
    card: {
        width: `calc(1.5 * ${width.popover})`,
    },
    figure: {
        display: "flex",
        alignItems: "baseline",
        gap: space[2],
        fontWeight: weight.semibold,
        fontVariantNumeric: "tabular-nums",
    },
    caption: {
        color: color.mutedForeground,
    },
});

/** A card of the notes written this month: the figure, its change and the months before it. */
export const chartCard = defineExample({
    of: Chart,
    name: "card",
    description: t`A card of the notes written this month: the figure, its change and the months before.`,
    render: () => (
        <Card xstyle={card.card}>
            <CardHeader>
                <CardDescription>Notes written in Work</CardDescription>
                <CardTitle>
                    <span {...style.attrs(text["2xl"], card.figure)}>
                        30
                        <Badge variant="secondary">+43%</Badge>
                    </span>
                </CardTitle>
            </CardHeader>
            <CardContent>
                <Chart
                    label="Notes written each month in Work"
                    height={120}
                    definition={defineChart({
                        marks: [
                            areaY(WORK, { x: "month", y: "notes", fill: `url(#${FADE.id})` }),
                            lineY(WORK, { x: "month", y: "notes", strokeWidth: 2 }),
                        ],
                        scales: {
                            x: { scale: () => scalePoint().padding(0) },
                            y: { scale: scaleLinear, nice: true },
                        },
                        guides: false,
                        gradients: [FADE],
                        tooltip,
                    })}
                />
                <p {...style.attrs(text.xs, card.caption)}>Up from 21 in March</p>
            </CardContent>
        </Card>
    ),
});

/** The notes written each month, stacked by notebook, with a legend. */
export const chartStackedBar = defineExample({
    of: Chart,
    name: "stackedBar",
    description: t`The notes written each month, stacked by notebook, with a legend.`,
    layout: "padded",
    render: () => (
        <Chart
            label="Notes written each month by notebook"
            height={240}
            xstyle={card.wide}
            definition={defineChart({
                marks: [
                    barY(WRITTEN, {
                        x: "month",
                        y: "notes",
                        color: "notebook",
                        radius: { end: 4, stack: "outer" },
                    }),
                ],
                scales: {
                    x: { scale: () => scaleBand().padding(0.4), axis: { label: "Month" } },
                    y: { scale: scaleLinear, nice: true, grid: true, axis: { label: "Notes" } },
                },
                color: { legend: colorLegend() },
                tooltip,
            })}
        />
    ),
});

/** The notes across every notebook, which the donut's middle reads out. */
const TOTAL = NOTEBOOKS.reduce((sum, notebook) => sum + notebook.notes, 0);

/** A donut of the share of each notebook, its total of notes in the middle. */
export const chartDonutTotal = defineExample({
    of: Chart,
    name: "donutTotal",
    description: t`A donut of the share of each notebook, its total of notes in the middle.`,
    render: () => (
        <div {...style.attrs(card.donut)}>
            <Chart
                label="Share of notes per notebook"
                description={`${String(TOTAL)} notes in all`}
                height={240}
                definition={defineChart({
                    marks: [
                        polar({
                            inset: 8,
                            marks: [
                                radialArc(pie(NOTEBOOKS, { value: "notes" }), {
                                    innerRadius: ({ radius }) => radius * 0.7,
                                    color: "notebook",
                                    key: "notebook",
                                }),
                            ],
                            scales: { angle: null, radius: null },
                        }),
                    ],
                    scales: { x: null, y: null },
                    tooltip,
                })}
            />
            <div aria-hidden="true" {...style.attrs(card.middle)}>
                <span {...style.attrs(text["2xl"], card.figure)}>{TOTAL}</span>
                <span {...style.attrs(text.xs, card.caption)}>notes</span>
            </div>
        </div>
    ),
});
