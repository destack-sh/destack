import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Card, CardDescription, CardHeader, CardTitle } from "../card/index.ts";
import { Grid } from "./grid.tsx";

/** A space's apps as cards in as many columns as fit. */
export const gridApps = defineExample({
    of: Grid,
    name: "apps",
    description: t`A space's apps as cards in as many columns as fit.`,
    properties: {
        min: "12rem",
    },
    render: (properties) => (
        <Grid {...properties}>
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

/** A space's apps in wider columns. */
export const gridAppsWide = defineExample({
    of: Grid,
    name: "appsWide",
    description: t`A space's apps in wider columns.`,
    properties: {
        min: "16rem",
    },
    render: (properties) => (
        <Grid {...properties}>
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
