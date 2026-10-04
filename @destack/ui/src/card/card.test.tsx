import { expect, test } from "@destack/test";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render each element of a card as a div with its data-slot", () => {
    const container = draw(() => (
        <Card aria-labelledby="title">
            <CardHeader>
                <CardTitle id="title">Storage</CardTitle>
                <CardDescription>2 of 5 GB used</CardDescription>
                <CardAction>Upgrade</CardAction>
            </CardHeader>
            <CardContent>Notes, files and photos</CardContent>
            <CardFooter>Renews monthly</CardFooter>
        </Card>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="card" aria-labelledby="title">' +
            '<div data-slot="card-header">' +
            '<div data-slot="card-title" id="title">Storage</div>' +
            '<div data-slot="card-description">2 of 5 GB used</div>' +
            '<div data-slot="card-action">Upgrade</div></div>' +
            '<div data-slot="card-content">Notes, files and photos</div>' +
            '<div data-slot="card-footer">Renews monthly</div></div>',
    );
});
