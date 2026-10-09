import { defineExample } from "@destack/package/declare";
import { Card, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Grid } from "./grid.tsx";

/** A space's apps as cards in as many columns as fit. */
export const gridApps = defineExample({
    of: Grid,
    name: "apps",
    description: "a space's apps as cards in as many columns as fit",
    render: () => (
        <Grid min="12rem">
            {[
                ["Notes", "142 notes"],
                ["Tasks", "18 open"],
                ["Pages", "6 published"],
                ["Calendar", "3 events today"],
            ].map(([title, description]) => (
                <Card>
                    <CardHeader>
                        <CardTitle>{title}</CardTitle>
                        <CardDescription>{description}</CardDescription>
                    </CardHeader>
                </Card>
            ))}
        </Grid>
    ),
});
