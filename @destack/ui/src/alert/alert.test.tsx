import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Alert, AlertDescription, AlertTitle } from "./index.ts";

test("announce an alert with its title and description", () => {
    const container = draw(() => (
        <Alert>
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>Free up space to keep syncing.</AlertDescription>
        </Alert>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="alert" data-variant="default" role="alert">' +
            '<div data-slot="alert-title">Sync paused</div>' +
            '<div data-slot="alert-description">Free up space to keep syncing.</div></div>',
    );
});

test("color a destructive alert's description apart from a default alert's", () => {
    const container = draw(() => (
        <>
            <Alert>
                <AlertDescription>Saved</AlertDescription>
            </Alert>
            <Alert variant="destructive">
                <AlertDescription>Not saved</AlertDescription>
            </Alert>
        </>
    ));
    const [saved, failed] = [...container.querySelectorAll("[data-slot=alert-description]")];
    expect(saved?.className).not.toBe(failed?.className);
});
