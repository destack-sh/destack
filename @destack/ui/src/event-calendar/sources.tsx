import { renderPart } from "../part/index.ts";
import * as style from "@destack/style";
import { radius, space, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { For, type JSX, omit } from "@destack/view";
import { Checkbox } from "../checkbox/index.ts";
import { Label } from "../label/index.ts";
import { DEFAULT_COLOR, type EventCalendarPartProperties, useEventCalendar } from "./control.ts";

/** The styles of a calendar's list of calendars. */
const styles = style.create({
    list: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
    },
    item: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    swatch: {
        width: space[2],
        height: space[2],
        borderRadius: radius.full,
        flexShrink: 0,
    },
});

/** The places of a calendar list's parts. */
const places = style.create({
    swatch: (mark: string) => ({ backgroundColor: mark }),
});

/** Render the calendars in their colors, each shown or hidden by its box. */
export function EventCalendarSourceList(properties: EventCalendarPartProperties): JSX.Element {
    // list each calendar with a box that shows or hides its events
    const control = useEventCalendar();
    const rest = omit(properties, "children");

    return renderPart("div", "event-calendar-source-list", rest, styles.list, {
        role: "list" as const,
        get children() {
            return (
                <For each={control.options.sources ?? []}>
                    {(source) => (
                        <div
                            role="listitem"
                            data-slot="event-calendar-source"
                            data-source={source.key}
                        >
                            <Label xstyle={[text.sm, styles.item]}>
                                <Checkbox
                                    checked={!control.hiddenSources().includes(source.key)}
                                    onCheckedChange={() => control.toggleSource(source.key)}
                                />
                                <span
                                    aria-hidden="true"
                                    {...style.attrs(
                                        styles.swatch,
                                        places.swatch(
                                            swatch[`${source.color ?? DEFAULT_COLOR}Mark`],
                                        ),
                                    )}
                                />
                                {source.label}
                            </Label>
                        </div>
                    )}
                </For>
            );
        },
    });
}
