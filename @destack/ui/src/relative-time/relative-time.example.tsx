import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { For, useEnvironment } from "@destack/view";
import { Stack } from "../stack/index.ts";
import { RelativeTime } from "./relative-time.tsx";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** The minutes from now of some moments. */
const MOMENTS = [0, -4, -90, -60 * 26, -60 * 24 * 9, 60 * 3, -60 * 24 * 60];

/** Moments near now, and one far off written as a date. */
export const relativeTimeMoments = defineExample({
    of: RelativeTime,
    name: "moments",
    description: t`Moments near now, and one far off written as a date.`,
    render: () => {
        const { clock } = useEnvironment();
        const now = clock.now();

        return (
            <Stack space="2">
                <For each={MOMENTS}>
                    {(minutes) => <RelativeTime value={now + minutes * MINUTE} />}
                </For>
            </Stack>
        );
    },
});

/** The same moments in the fewest letters, such as beside a message. */
export const relativeTimeNarrow = defineExample({
    of: RelativeTime,
    name: "narrow",
    description: t`The same moments in the fewest letters, such as beside a message.`,
    render: () => {
        const { clock } = useEnvironment();
        const now = clock.now();

        return (
            <Stack space="2">
                <For each={MOMENTS}>
                    {(minutes) => (
                        <RelativeTime
                            value={now + minutes * MINUTE}
                            length="narrow"
                            format={{ month: "short", day: "numeric" }}
                        />
                    )}
                </For>
            </Stack>
        );
    },
});
