import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import {
    Activity,
    ActivityAddon,
    ActivityBadge,
    ActivityBody,
    ActivityBreak,
    ActivityItem,
} from "./index.ts";

test("render what happened as a list, its break left out of it", () => {
    const { container } = render(() => (
        <Activity aria-label="Activity">
            <ActivityItem condensed>
                <ActivityBadge color="green">*</ActivityBadge>
                <ActivityBody>Ada finished it</ActivityBody>
                <ActivityAddon>Undo</ActivityAddon>
            </ActivityItem>
            <ActivityBreak />
        </Activity>
    ));

    expect(markup(container)).toBe(
        [
            '<ol data-slot="activity" aria-label="Activity">',
            '<li data-slot="activity-item" data-condensed="">',
            '<div data-slot="activity-badge" style="--x-backgroundColor: var(--destack-swatch-green-tint); --x-color: var(--destack-swatch-green-tint-text);">*</div>',
            '<div data-slot="activity-body">Ada finished it</div>',
            '<div data-slot="activity-addon">Undo</div></li>',
            '<li data-slot="activity-break" role="presentation"></li></ol>',
        ].join(""),
    );
});
