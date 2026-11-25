import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Meter, MeterLabel, MeterTrack, MeterValue } from "./index.ts";

/** Read a container's meters as assistive technology does, with their levels. */
function read(container: Element): (string | null | undefined)[][] {
    return [...container.querySelectorAll("[role=meter]")].map((meter) => [
        container.querySelector(`#${meter.getAttribute("aria-labelledby") ?? "none"}`)?.textContent,
        meter.getAttribute("aria-valuenow"),
        meter.getAttribute("aria-valuetext"),
        meter.getAttribute("data-level"),
        meter.querySelector("[data-slot=meter-value]")?.textContent,
    ]);
}

test("write and read a value in its range, by its label and format", () => {
    const { container } = render(() => (
        <>
            <Meter value={30} max={120}>
                <MeterLabel>Seats</MeterLabel>
                <MeterValue />
                <MeterTrack />
            </Meter>
            <Meter
                value={60}
                max={50}
                format={{ style: "unit", unit: "gigabyte" }}
                getValueLabel={(written) => `${written} of 50`}
            >
                <MeterLabel>Storage</MeterLabel>
                <MeterValue>{(written, value) => `${written} (${String(value)})`}</MeterValue>
            </Meter>
        </>
    ));

    expect(read(container)).toEqual([
        ["Seats", "30", "25%", "optimum", "25%"],
        ["Storage", "50", "60 GB of 50", "optimum", "60 GB (50)"],
    ]);
});

/** Read the level of a value whose low part ends at 30 and whose high part starts at 70. */
function level(value: number, optimum: number): string | null {
    const { container } = render(() => (
        <Meter value={value} low={30} high={70} optimum={optimum} />
    ));

    return container.querySelector("[role=meter]")?.getAttribute("data-level") ?? null;
}

test("grade a value by the part of the range it is in, as HTML's meter does", () => {
    // used storage is best at none, a battery is best when full, and a temperature in between
    expect(
        [0, 100, 50].map((optimum) => [10, 50, 90].map((value) => level(value, optimum))),
    ).toEqual([
        ["optimum", "suboptimum", "critical"],
        ["critical", "suboptimum", "optimum"],
        ["suboptimum", "optimum", "suboptimum"],
    ]);
});
