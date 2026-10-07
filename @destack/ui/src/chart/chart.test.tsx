import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Chart } from "./chart.tsx";
import { defineChart, lineY } from "./index.ts";
import { scaleLinear } from "./module/scales/linear.ts";
import { scalePoint } from "./module/scales/point.ts";

/** The notes written each month in two notebooks. */
const WRITTEN = [
    { month: "Jan", notebook: "Work", notes: 18 },
    { month: "Feb", notebook: "Work", notes: 24 },
    { month: "Jan", notebook: "Trips", notes: 6 },
    { month: "Feb", notebook: "Trips", notes: 4 },
] as const;

/** The scales of a chart of notes by month. */
const SCALES = {
    x: { scale: () => scalePoint() },
    y: { scale: scaleLinear },
} as const;

/** List the stroke of each line a container draws, in order. */
function strokes(container: Element): (string | null)[] {
    return [...container.querySelectorAll("[data-ts-key^='line-0'] > path")].map((path) =>
        path.getAttribute("stroke"),
    );
}

test("draw a chart's series in the theme's chart colors under its name, unless its definition sets a palette", () => {
    const themed = draw(() => (
        <Chart
            label="Notes written each month"
            description="Notes per month in Work and Trips."
            definition={defineChart({
                marks: [lineY(WRITTEN, { x: "month", y: "notes", z: "notebook" })],
                scales: SCALES,
            })}
        />
    ));
    const own = draw(() => (
        <Chart
            label="Notes written each month"
            definition={defineChart({
                marks: [lineY(WRITTEN, { x: "month", y: "notes", z: "notebook" })],
                scales: SCALES,
                theme: { palette: ["#111111", "#222222"] },
            })}
        />
    ));
    const svg = themed.querySelector("svg");
    expect([
        ["role", "aria-roledescription", "aria-label", "tabindex"].map((name) =>
            svg?.getAttribute(name),
        ),
        svg?.querySelector("desc")?.textContent,
        strokes(themed),
        strokes(own),
    ]).toEqual([
        ["img", "chart", "Notes written each month", "0"],
        "Notes per month in Work and Trips.",
        ["var(--destack-color-chart1)", "var(--destack-color-chart2)"],
        ["#111111", "#222222"],
    ]);
});

test("read out the source rows as a table captioned by the chart's name, formatting the columns that format", () => {
    const container = draw(() => (
        <Chart
            label="Notes in Work"
            definition={defineChart({
                marks: [lineY(WRITTEN.slice(0, 2), { x: "month", y: "notes" })],
                scales: SCALES,
            })}
            table={{
                rows: WRITTEN.slice(0, 2),
                columns: [
                    { key: "month", label: "Month" },
                    { key: "notes", label: "Notes", format: (notes) => `${String(notes)} notes` },
                ],
            }}
        />
    ));
    expect(markup(container.querySelector("[data-slot=chart-table]") ?? container)).toBe(
        "<caption>Notes in Work</caption>" +
            '<thead><tr><th scope="col">Month</th><th scope="col">Notes</th></tr></thead>' +
            "<tbody><tr><td>Jan</td><td>18 notes</td></tr><tr><td>Feb</td><td>24 notes</td></tr></tbody>",
    );
});
