import { defineExample } from "@destack/package/declare";
import { Card, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Switcher } from "./switcher.tsx";

/** Three plans side by side, stacked once the container is narrower than 40rem. */
export const switcherPlans = defineExample({
    of: Switcher,
    name: "plans",
    description: "three plans side by side, stacked once the container is narrower than 40rem",
    render: () => (
        <Switcher threshold="40rem">
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
