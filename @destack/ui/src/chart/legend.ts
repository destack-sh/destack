import * as charts from "@tanstack/charts";

/** The gap between a legend's entries in pixels, two steps of the theme's space. */
const ITEM_GAP = 16;

/** Render a legend of a chart's colors, its entries side by side from the start. */
export function colorLegend<Value extends charts.ChartKey = charts.ChartKey>(
    options: charts.ColorLegendOptions<Value> = {},
): charts.ChartColorLegend {
    return charts.colorLegend({
        items: charts.colorLegendItems({ justify: "start", gap: ITEM_GAP }),
        ...options,
    });
}
