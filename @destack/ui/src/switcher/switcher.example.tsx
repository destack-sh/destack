import { defineExample } from "@destack/package/declare";
import { Card, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Switcher } from "./switcher.tsx";
import "./switcher.css";

/** Three plans side by side, stacked once the container is narrower than 40rem. */
export const switcherPlans = defineExample({
    of: Switcher,
    name: "plans",
    description: "three plans side by side, stacked once the container is narrower than 40rem",
    properties: {
        threshold: "40rem",
    },
    render: (properties) => (
        <Switcher {...properties}>
            {[
                ["Personal", "Free"],
                ["Team", "$8 per seat"],
                ["Hosted", "$20 per month"],
            ].map(([title, price]) => (
                <Card>
                    <CardHeader>
                        <CardTitle>{title}</CardTitle>
                        <CardDescription>{price}</CardDescription>
                    </CardHeader>
                </Card>
            ))}
        </Switcher>
    ),
});

/** The plans stacked below a wider threshold. */
export const switcherPlansStacked = defineExample({
    of: Switcher,
    name: "plans-stacked",
    description: "the plans stacked below a wider threshold",
    properties: {
        threshold: "60rem",
    },
    render: (properties) => (
        <Switcher {...properties}>
            {[
                ["Personal", "Free"],
                ["Team", "$8 per seat"],
                ["Hosted", "$20 per month"],
            ].map(([title, price]) => (
                <Card>
                    <CardHeader>
                        <CardTitle>{title}</CardTitle>
                        <CardDescription>{price}</CardDescription>
                    </CardHeader>
                </Card>
            ))}
        </Switcher>
    ),
});
