import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Stack } from "../stack/index.ts";
import { Meter, MeterLabel, MeterTrack, MeterValue } from "./meter.tsx";

/** Storage used in a plan, getting full and nearly full. */
export const meterStorage = defineExample({
    of: Meter,
    name: "storage",
    description: t`Storage used in a plan, getting full and nearly full.`,
    layout: "padded",
    render: () => (
        <Stack space="6">
            {[12, 41, 48].map((used) => (
                <Meter
                    value={used}
                    max={50}
                    low={35}
                    high={45}
                    optimum={0}
                    format={{ style: "unit", unit: "gigabyte" }}
                >
                    <MeterLabel>Storage</MeterLabel>
                    <MeterValue>{(written) => `${written} of 50 GB`}</MeterValue>
                    <MeterTrack />
                </Meter>
            ))}
        </Stack>
    ),
});

/** A battery's charge, good when high, its bar alone. */
export const meterBattery = defineExample({
    of: MeterTrack,
    name: "battery",
    description: t`A battery's charge, good when high, its bar alone.`,
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Meter aria-label="Battery" value={82} low={20} high={40} optimum={100} />
            <Meter aria-label="Spare battery" value={8} low={20} high={40} optimum={100} />
        </Stack>
    ),
});
