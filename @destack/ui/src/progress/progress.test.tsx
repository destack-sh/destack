import { Errored } from "@destack/view";
import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Progress, ProgressIndicator } from "./index.ts";

test("report a progress bar's value, state and percentage, and an indeterminate one without a value", () => {
    const container = draw(() => (
        <>
            <Progress value={30} aria-label="Upload">
                <ProgressIndicator />
            </Progress>
            <Progress value={140} max={140} aria-label="Import" />
            <Progress aria-label="Sync" />
        </>
    ));

    expect(markup(container)).toBe(
        '<div role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="30" aria-valuetext="30%" data-slot="progress" data-state="loading" data-value="30" data-max="100" aria-label="Upload"><div data-slot="progress-indicator" data-state="loading" data-value="30" data-max="100" style="--x-transform: translateX(-70%);"></div></div>' +
            '<div role="progressbar" aria-valuemin="0" aria-valuemax="140" aria-valuenow="140" aria-valuetext="100%" data-slot="progress" data-state="complete" data-value="140" data-max="140" aria-label="Import"><div data-slot="progress-indicator" data-state="complete" data-value="140" data-max="140" style="--x-transform: translateX(-0%);"></div></div>' +
            '<div role="progressbar" aria-valuemin="0" aria-valuemax="100" data-slot="progress" data-state="indeterminate" data-max="100" aria-label="Sync"><div data-slot="progress-indicator" data-state="indeterminate" data-max="100"></div></div>',
    );
});

test("refuse a progress bar whose maximum is not positive", () => {
    const container = draw(() => (
        <Errored fallback={(error) => String(error())}>
            <Progress value={0} max={0} aria-label="Upload" />
        </Errored>
    ));

    expect(container.textContent).toBe(
        "RangeError: a progress bar's maximum must be positive, not 0",
    );
});
