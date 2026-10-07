import { defineExample } from "@destack/package/declare";
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

/** The notes each notebook holds. */
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
    description: "a line per notebook of the notes written each month, with a tooltip and a legend",
    render: () => (
        <Chart
            label="Notes written each month by notebook"
            definition={defineChart({
                marks: [lineY(WRITTEN, { x: "month", y: "notes", z: "notebook" })],
                scales: {
                    x: MONTHS,
                    y: { scale: scaleLinear, nice: true, grid: true },
                },
                color: { legend: colorLegend({ label: "Notebook" }) },
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
    description: "ten thousand readings drawn as dots on a canvas, as large datasets draw",
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
    description: "the notes written each month in the work notebook as a filled area",
    render: () => (
        <Chart
            label="Notes written each month in Work"
            height={240}
            definition={defineChart({
                marks: [
                    areaY(
                        WRITTEN.filter((row) => row.notebook === "Work"),
                        { x: "month", y: "notes", fillOpacity: 0.3 },
                    ),
                ],
                scales: {
                    x: MONTHS,
                    y: { scale: scaleLinear, nice: true, grid: true },
                },
                tooltip,
            })}
        />
    ),
});

/** A bar per notebook of the notes it holds. */
export const chartBar = defineExample({
    of: Chart,
    name: "bar",
    description: "a bar per notebook of the notes it holds",
    render: () => (
        <Chart
            label="Notes per notebook"
            aspectRatio={16 / 9}
            definition={defineChart({
                marks: [barY(NOTEBOOKS, { x: "notebook", y: "notes" })],
                scales: {
                    x: {
                        scale: () => scaleBand().padding(0.2),
                        axis: { label: "Notebook" },
                    },
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
    description: "a dot per note of its length against its edits",
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

/** A donut of the share of notes each notebook holds. */
export const chartPie = defineExample({
    of: Chart,
    name: "pie",
    description: "a donut of the share of notes each notebook holds",
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
                color: { legend: colorLegend({ label: "Notebook" }) },
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
