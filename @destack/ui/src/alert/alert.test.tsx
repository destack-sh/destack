import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Alert, AlertAction, AlertClose, AlertDescription, AlertTitle } from "./index.ts";

test("announce an alert with its title and description", () => {
    const { container } = render(() => (
        <Alert>
            <AlertTitle>Sync paused</AlertTitle>
            <AlertDescription>Free up space to keep syncing.</AlertDescription>
        </Alert>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="alert" data-variant="default" role="status">' +
            '<div data-slot="alert-title">Sync paused</div>' +
            '<div data-slot="alert-description">Free up space to keep syncing.</div></div>',
    );
});

test("mark each alert with its variant, which its colors follow", () => {
    const { container } = render(() => (
        <>
            <Alert>
                <AlertDescription>Saved</AlertDescription>
            </Alert>
            <Alert variant="destructive">
                <AlertDescription>Not saved</AlertDescription>
            </Alert>
        </>
    ));
    const [saved, failed] = [...container.querySelectorAll<HTMLElement>("[data-slot=alert]")];

    // each alert marks its variant
    expect([saved?.dataset["variant"], failed?.dataset["variant"]]).toEqual([
        "default",
        "destructive",
    ]);
});

test("interrupt for a warning or a failure, and announce the other statuses politely", () => {
    const variants = ["default", "info", "success", "warning", "destructive"] as const;
    const { container } = render(() =>
        variants.map((variant) => (
            <Alert variant={variant}>
                <AlertTitle>{variant}</AlertTitle>
            </Alert>
        )),
    );
    const roles = [...container.querySelectorAll("[data-slot=alert]")].map((alert) => [
        alert.textContent,
        alert.getAttribute("role"),
    ]);

    expect(roles).toEqual([
        ["default", "status"],
        ["info", "status"],
        ["success", "status"],
        ["warning", "alert"],
        ["destructive", "alert"],
    ]);
});

test("end an alert with the owner's actions and a dismiss button that hands the owner the click", () => {
    // render a failure with a retry, then dismiss it
    const dismissed: string[] = [];
    const { container } = render(() => (
        <Alert variant="destructive">
            <AlertTitle>Couldn't save</AlertTitle>
            <AlertAction>
                <button type="button">Retry</button>
            </AlertAction>
            <AlertClose onClick={() => dismissed.push("dismissed")} />
        </Alert>
    ));
    const close = container.querySelector<HTMLButtonElement>("[data-slot=alert-close]");
    close?.click();

    expect({
        action: container.querySelector("[data-slot=alert-action]")?.textContent,
        label: close?.getAttribute("aria-label"),
        dismissed,
    }).toEqual({ action: "Retry", label: "Dismiss", dismissed: ["dismissed"] });
});
