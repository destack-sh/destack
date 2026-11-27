import starIcon from "@destack/icon/phosphor/star";
import { plural, t } from "@destack/locale";
import { Rating } from "@destack/schema";
import { For, type JSX, useLocale } from "@destack/view";
import { Icon } from "../icon/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The most stars a rating shows where its schema sets no most, as rating widgets show five. */
const STARS = 5;

/** Render the input of a rating: a row of stars, pressing the one given again clearing it. */
export function RatingInput(properties: NamedValueInputProperties): JSX.Element {
    // read the rating and the most stars its schema allows
    const locale = useLocale();
    const rating = (): number => readValue(properties.value, Rating) ?? 0;
    const most = (): number => {
        const maximum = properties.type.schema["maximum"];

        return typeof maximum === "number" ? maximum : STARS;
    };
    const stars = (): readonly number[] => Array.from({ length: most() }, (_, index) => index + 1);

    return (
        <ToggleGroup
            size="sm"
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            value={rating() === 0 ? undefined : String(rating())}
            onValueChange={(key) => properties.onValueChange(key === undefined ? 0 : Number(key))}
        >
            <For each={stars()}>
                {(star) => (
                    <ToggleGroupItem
                        value={String(star)}
                        aria-label={locale.render(
                            t.context(
                                "option",
                            )`${plural(star, { one: "# star", other: "# stars" })}`,
                        )}
                    >
                        <Icon icon={starIcon} weight={star <= rating() ? "fill" : "regular"} />
                    </ToggleGroupItem>
                )}
            </For>
        </ToggleGroup>
    );
}
