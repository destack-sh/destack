import { Icon } from "../icon/index.ts";
import playIcon from "@destack/icon/phosphor/play";
import { t } from "@destack/locale";
import { Motion, MOTION_STATES } from "@destack/style";
import * as style from "@destack/style";
import { color, radius, size, space } from "@destack/theme/tokens.stylex";
import { createSignal, createUniqueId, For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Toggle } from "../toggle/index.ts";

/** The styles of a motion stage. */
const styles = style.create({
    stage: {
        display: "grid",
        placeItems: "center",
        height: "6rem",
        borderRadius: radius[2],
        backgroundColor: color.muted,
        overflow: "hidden",
    },
    sample: {
        width: size[3],
        height: size[3],
        borderRadius: radius[2],
        backgroundColor: color.foreground,
    },
    controls: {
        display: "flex",
        gap: space[1],
    },
});

/** Render a sample moving as a motion does, with buttons to play it again, open and hide it. */
export function MotionStage(properties: {
    /** The motion. */
    readonly motion: Motion;
}): JSX.Element {
    // style a sample of its own, played again as its key changes
    const locale = useLocale();
    const name = `motion-${createUniqueId().replaceAll(/[^\w-]/gu, "")}`;
    const [play, setPlay] = createSignal(0);
    const [isOpen, setOpen] = createSignal(false);
    const [isHidden, setHidden] = createSignal(false);
    const css = (): string => Motion.css(properties.motion, `[data-motion="${name}"]`, name);

    return (
        <>
            <style>{css()}</style>
            <div {...style.attrs(styles.stage)}>
                <For each={[play()]}>
                    {() => (
                        <div
                            data-motion={name}
                            data-open={isOpen() ? "" : undefined}
                            data-closed={isHidden() ? "" : undefined}
                            hidden={isHidden()}
                            {...style.attrs(styles.sample)}
                        />
                    )}
                </For>
            </div>
            <div {...style.attrs(styles.controls)}>
                <Button
                    variant="ghost"
                    size="sm"
                    isIconOnly
                    aria-label={locale.render(t.context("action")`Play again`)}
                    onClick={() => setPlay((count) => count + 1)}
                >
                    <Icon icon={playIcon} />
                </Button>
                <Toggle size="sm" pressed={isOpen()} onPressedChange={setOpen}>
                    {locale.render(MOTION_STATES.open.title)}
                </Toggle>
                <Toggle size="sm" pressed={isHidden()} onPressedChange={setHidden}>
                    {locale.render(t.context("action")`Hide`)}
                </Toggle>
            </div>
        </>
    );
}
