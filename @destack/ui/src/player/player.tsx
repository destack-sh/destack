import captionsIcon from "@destack/icon/phosphor/subtitles";
import cornersInIcon from "@destack/icon/phosphor/corners-in";
import cornersOutIcon from "@destack/icon/phosphor/corners-out";
import pauseIcon from "@destack/icon/phosphor/pause";
import pictureIcon from "@destack/icon/phosphor/picture-in-picture";
import playIcon from "@destack/icon/phosphor/play";
import replayIcon from "@destack/icon/phosphor/arrow-counter-clockwise";
import skipBackIcon from "@destack/icon/phosphor/skip-back";
import skipForwardIcon from "@destack/icon/phosphor/skip-forward";
import speakerHighIcon from "@destack/icon/phosphor/speaker-high";
import speakerLowIcon from "@destack/icon/phosphor/speaker-low";
import speakerOffIcon from "@destack/icon/phosphor/speaker-x";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    For,
    handle,
    type JSX,
    merge,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { createIdleTimer } from "@destack/view/primitives/idle";
import { ActionButton, type ActionButtonProperties } from "../action/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "../dropdown-menu/index.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { Slider, SliderIndicator, SliderThumb, SliderTrack } from "../slider/index.ts";
import { Spinner } from "../spinner/index.ts";
import {
    nextTrack,
    previousTrack,
    seekTo,
    skipBack,
    skipForward,
    toggleCaptions,
    toggleFullscreen,
    toggleMute,
    togglePictureInPicture,
    togglePlay,
    volumeDown,
    volumeUp,
} from "./actions.ts";
import {
    PlayerContext,
    PlayerControl,
    type PlayerSessionOptions,
    SKIP,
    usePlayer,
} from "./control.ts";

/** How long the bar stays over a film once the pointer stands still, in milliseconds. */
const IDLE = 2500;

/** How far an arrow moves along a film, in seconds. */
const SEEK_STEP = 5;

/** How fine a pointer moves along a film, in seconds. */
const SEEK_GRAIN = 0.1;

/** The speeds a person chooses among, as Video.js's. */
const RATES = [0.5, 0.75, 1, 1.25, 1.5, 2] as const;

/** The shade behind what floats over a film. */
const SHADE = "color-mix(in oklab, black 60%, transparent)";

/** The shade behind captions. */
const CAPTION_SHADE = "color-mix(in oklab, black 75%, transparent)";

/** A film too narrow for every part of its bar. */
const NARROW = "@container (max-width: 30rem)";

/** The styles of a player and its parts. */
const styles = style.create({
    player: {
        width: "100%",
        minWidth: 0,
        outlineStyle: "none",
    },
    frame: {
        position: "relative",
        isolation: "isolate",
        display: "flex",
        overflow: "hidden",
        width: "100%",
        borderRadius: { default: radius[4], ":fullscreen": 0 },
        backgroundColor: "black",
        color: color.foreground,
        colorScheme: "dark",
        outlineStyle: "none",
        containerType: "inline-size",
    },
    ratio: (ratio: number) => ({ aspectRatio: ratio }),
    whole: {
        aspectRatio: "auto",
    },
    alone: {
        cursor: "none",
    },
    film: {
        display: "block",
        width: "100%",
        height: "100%",
        objectFit: "contain",
    },
    over: {
        position: "absolute",
        inset: 0,
        display: "grid",
        placeItems: "center",
        pointerEvents: "none",
    },
    fading: {
        transitionProperty: "opacity",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    hidden: {
        opacity: { default: 0, ":is(:focus-visible, :has(:focus-visible))": 1 },
        pointerEvents: {
            default: "none",
            ":is(:focus-visible, :has(:focus-visible))": "auto",
        },
    },
    start: {
        width: space[8],
        height: space[8],
        borderRadius: radius.full,
        borderWidth: 0,
        backgroundColor: {
            default: SHADE,
            ":hover": {
                default: null,
                [media.hover]: "color-mix(in oklab, black 75%, transparent)",
            },
        },
        backdropFilter: "blur(16px)",
        color: color.foreground,
        fontSize: `calc(7 * ${space[1]})`,
        pointerEvents: "auto",
    },
    captions: {
        position: "absolute",
        insetInline: space[4],
        insetBlockEnd: space[4],
        display: "grid",
        justifyItems: "center",
        textAlign: "center",
        pointerEvents: "none",
        transitionProperty: "transform",
        transitionDuration: { default: motion.durationMedium, [media.motionReduce]: "0s" },
        transitionTimingFunction: motion.easingStandard,
    },
    raised: {
        transform: `translateY(calc(-1 * (${size[2]} + ${space[7]})))`,
    },
    caption: {
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[1],
        backgroundColor: { default: CAPTION_SHADE, [media.forcedColors]: "Canvas" },
        fontSize: `clamp(${space[3]}, 3cqw, calc(7 * ${space[1]}))`,
        lineHeight: 1.35,
        whiteSpace: "pre-line",
    },
    bar: {
        position: "absolute",
        insetInline: 0,
        insetBlockEnd: 0,
        display: "grid",
        gap: space[1],
        paddingBlockStart: space[7],
        paddingBlockEnd: space[2],
        paddingInline: space[3],
        backgroundImage: `linear-gradient(transparent, ${SHADE})`,
    },
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    space: {
        flex: 1,
    },
    time: {
        flexShrink: 0,
        paddingInline: space[2],
        fontVariantNumeric: "tabular-nums",
        whiteSpace: "nowrap",
    },
    muted: {
        color: color.mutedForeground,
    },
    icon: {
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
            [media.forcedColors]: "ButtonFace",
        },
        fontSize: `calc(4.5 * ${space[1]})`,
    },
    seek: {
        position: "relative",
        flex: 1,
        minWidth: 0,
    },
    loaded: (share: string) => ({
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: 0,
        width: share,
        backgroundColor: color.foreground,
        opacity: 0.25,
    }),
    preview: (share: string) => ({
        position: "absolute",
        insetBlockEnd: "100%",
        insetInlineStart: share,
        transform: "translateX(-50%)",
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
        fontVariantNumeric: "tabular-nums",
        pointerEvents: "none",
    }),
    volume: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    level: {
        display: { default: "grid", [media.pointerCoarse]: "none", [NARROW]: "none" },
        width: space[9],
    },
    // as wide as its widest, so no other part moves
    speed: {
        minWidth: `calc(5ch + 4 * ${space[1]})`,
        paddingInline: space[1],
        fontWeight: "inherit",
        fontVariantNumeric: "tabular-nums",
    },
    failure: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    spare: {
        display: { default: "inline-flex", [NARROW]: "none" },
    },
    centered: {
        position: "absolute",
        inset: 0,
        display: "grid",
        alignContent: "center",
        justifyItems: "center",
        textAlign: "center",
    },
    audio: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        width: "100%",
        minWidth: 0,
        padding: space[2],
        paddingInlineEnd: space[3],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.card,
        color: color.foreground,
    },
    picture: {
        flexShrink: 0,
        width: space[8],
        height: space[8],
        borderRadius: radius[2],
        objectFit: "cover",
    },
    words: {
        display: "grid",
        flexGrow: 1,
        flexBasis: space[9],
        gap: space[1],
        minWidth: `min(100%, 12rem)`,
    },
    line: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    title: {
        fontWeight: weight.medium,
    },
});

/** Write a time in seconds as a clock does, such as 1:05 or 1:02:05. */
function clockTime(seconds: number, locale: Localization): string {
    // split it into hours, minutes and seconds
    const whole = Math.floor(seconds);
    const hours = Math.floor(whole / 3600);
    const minutes = Math.floor(whole / 60) % 60;
    const two = (value: number) =>
        locale.number(value, { minimumIntegerDigits: 2, useGrouping: false });
    const last = two(whole % 60);

    return hours > 0
        ? `${locale.number(hours)}:${two(minutes)}:${last}`
        : `${locale.number(minutes)}:${last}`;
}

/** Write a time in seconds as it is said, such as 1 minute, 5 seconds. */
function spokenTime(seconds: number, locale: Localization): string {
    // leave out the units it has none of
    const whole = Math.floor(seconds);
    const units = [
        ["hour", Math.floor(whole / 3600)],
        ["minute", Math.floor(whole / 60) % 60],
        ["second", whole % 60],
    ] as const;
    const said = units.filter(([unit, value]) => value > 0 || (unit === "second" && whole === 0));

    return locale.list(
        said.map(([unit, value]) =>
            locale.number(value, { style: "unit", unit, unitDisplay: "long" }),
        ),
    );
}

/** The properties of an element of a player, with its own attributes. */
export type PlayerElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "title"> &
    ElementPartProperties;

/** The properties of a button of a player, which runs an action there. */
export type PlayerButtonProperties = Omit<
    ActionButtonProperties,
    "action" | "element" | "href" | "download"
>;

/** Offer what a person does with a film or a sound as actions. */
function handlersOf(control: PlayerControl): ((element: Element) => void)[] {
    const isFilm = (): boolean => control.element() instanceof HTMLVideoElement;

    return [
        handle(togglePlay, { isOn: () => !control.isPaused(), run: () => control.toggle() }),
        handle(skipBack, { run: () => control.skip(-SKIP) }),
        handle(skipForward, { run: () => control.skip(SKIP) }),
        handle(toggleMute, { isOn: control.isMuted, run: () => control.toggleMute() }),
        handle(volumeUp, { isEnabled: isFilm, run: () => control.turn(1) }),
        handle(volumeDown, { isEnabled: isFilm, run: () => control.turn(-1) }),
        handle(seekTo, { run: ({ values }) => control.seek(values.time) }),
        handle(toggleCaptions, {
            isEnabled: () => control.captions().length > 0,
            isOn: () => control.captions().some(({ isShown }) => isShown),
            run: () => control.toggleCaptions(),
        }),
        handle(toggleFullscreen, {
            isEnabled: () => isFilm() && control.canFullscreen(),
            isOn: control.isFullscreen,
            run: () => control.toggleFullscreen(),
        }),
        handle(togglePictureInPicture, {
            isEnabled: () => control.canPictureInPicture(),
            isOn: control.isPictureInPicture,
            run: () => control.togglePictureInPicture(),
        }),
    ];
}

/** Render the button that plays a film or a sound, or pauses it. */
export function PlayerPlayButton(properties: PlayerButtonProperties): JSX.Element {
    const control = usePlayer();
    const icon = () => (control.isEnded() ? replayIcon : control.isPaused() ? playIcon : pauseIcon);

    return (
        <ActionButton
            data-slot="player-play-button"
            variant="ghost"
            isIconOnly
            weight="fill"
            icon={icon()}
            {...omit(properties, "xstyle")}
            action={togglePlay}
            xstyle={[styles.icon, properties.xstyle]}
        />
    );
}

/** Render where a film or a sound is, as a slider that moves it. */
export function PlayerSeekBar(properties: PlayerElementProperties): JSX.Element {
    // show the time under the pointer
    const control = usePlayer();
    const locale = useLocale();
    const [preview, setPreview] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    const length = (): number => control.length() ?? 0;
    const share = (time: number): string =>
        `${String(length() === 0 ? 0 : Math.min(time / length(), 1) * 100)}%`;
    const move = (event: PointerEvent & { readonly currentTarget: HTMLElement }): void => {
        const { left, width } = event.currentTarget.getBoundingClientRect();
        const at = Math.min(Math.max((event.clientX - left) / width, 0), 1);
        setPreview(event.pointerType === "touch" ? undefined : at * length());
    };

    return renderPart("div", "player-seek-bar", properties, styles.seek, {
        // run time from left to right in every language, as players do
        dir: "ltr",
        onPointerMove: move,
        onPointerLeave: () => setPreview(undefined),
        get children() {
            return (
                <>
                    <Slider
                        aria-label={locale.render(t.context("label")`Position`)}
                        value={[Math.min(control.time(), length())]}
                        max={length() === 0 ? 1 : length()}
                        step={SEEK_GRAIN}
                        disabled={control.length() === undefined}
                        onValueChange={([time = 0]) => control.seek(time)}
                    >
                        <SliderTrack>
                            <span {...style.attrs(styles.loaded(share(control.loaded())))} />
                            <SliderIndicator />
                        </SliderTrack>
                        <SliderThumb
                            aria-valuetext={spokenTime(control.time(), locale)}
                            onKeyDown={(event: KeyboardEvent) => {
                                // move by some seconds rather than the finest step
                                const steps: Readonly<Record<string, number>> = {
                                    ArrowLeft: -SEEK_STEP,
                                    ArrowDown: -SEEK_STEP,
                                    ArrowRight: SEEK_STEP,
                                    ArrowUp: SEEK_STEP,
                                };
                                const step = steps[event.key];
                                if (step !== undefined) {
                                    event.preventDefault();
                                    control.skip(step);
                                } else if (event.key === " ") {
                                    event.preventDefault();
                                    control.toggle();
                                }
                            }}
                        />
                    </Slider>
                    <Show when={control.length() === undefined ? undefined : preview()}>
                        {(time) => (
                            <span
                                aria-hidden="true"
                                {...style.attrs(text.xs, styles.preview(share(time())))}
                            >
                                {clockTime(time(), locale)}
                            </span>
                        )}
                    </Show>
                </>
            );
        },
    });
}

/** Render where a film or a sound is and how long it is. */
export function PlayerTime(properties: PlayerElementProperties): JSX.Element {
    const control = usePlayer();
    const locale = useLocale();

    return renderPart("div", "player-time", properties, [text.xs, styles.time], {
        dir: "ltr",
        get children() {
            return (
                <>
                    {clockTime(control.time(), locale)}
                    <Show when={control.length()}>
                        {(length) => (
                            <span {...style.attrs(styles.muted)}>
                                {" / "}
                                {clockTime(length(), locale)}
                            </span>
                        )}
                    </Show>
                </>
            );
        },
    });
}

/** Render the button that mutes a sound, and how loud it is where a pointer sets it. */
export function PlayerVolume(properties: PlayerElementProperties): JSX.Element {
    // show how loud it is
    const control = usePlayer();
    const locale = useLocale();
    const volume = (): number => (control.isMuted() ? 0 : control.volume());
    const icon = () =>
        volume() === 0 ? speakerOffIcon : volume() < 0.5 ? speakerLowIcon : speakerHighIcon;

    return renderPart("div", "player-volume", properties, styles.volume, {
        get children() {
            return (
                <>
                    <ActionButton
                        data-slot="player-mute-button"
                        variant="ghost"
                        isIconOnly
                        icon={icon()}
                        action={toggleMute}
                        xstyle={styles.icon}
                    />
                    <Slider
                        dir="ltr"
                        aria-label={locale.render(t.context("label")`Volume`)}
                        value={[Math.round(volume() * 100)]}
                        step={5}
                        onValueChange={([level = 0]) => control.setVolume(level / 100)}
                        xstyle={styles.level}
                    />
                </>
            );
        },
    });
}

/** Render the menu of how fast a film or a sound plays. */
export function PlayerSpeed(properties: ButtonProperties): JSX.Element {
    // write a speed as a factor
    const control = usePlayer();
    const locale = useLocale();
    const factor = (rate: number): string => `${locale.number(rate)}×`;

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        data-slot="player-speed"
                        variant="ghost"
                        aria-label={`${locale.render(t.context("label")`Speed`)} ${factor(control.rate())}`}
                        {...omit(properties, "xstyle", "children")}
                        xstyle={[styles.icon, text.xs, styles.speed, properties.xstyle]}
                    />
                )}
            >
                {factor(control.rate())}
            </DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuRadioGroup
                    aria-label={locale.render(t.context("label")`Speed`)}
                    value={String(control.rate())}
                    onValueChange={(rate) => control.setRate(Number(rate))}
                >
                    <For each={RATES}>
                        {(rate) => (
                            <DropdownMenuRadioItem value={String(rate)}>
                                {factor(rate)}
                            </DropdownMenuRadioItem>
                        )}
                    </For>
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** Render what shows a film's captions: a button, or a menu of several, nothing without any. */
export function PlayerCaptionsButton(properties: PlayerButtonProperties): JSX.Element {
    // choose among several
    const control = usePlayer();
    const locale = useLocale();
    const shown = (): number => control.captions().findIndex(({ isShown }) => isShown);
    const drawn = () => (shown() === -1 ? "regular" : "fill");

    return (
        <Show when={control.captions().length > 0}>
            <Show
                when={control.captions().length > 1}
                fallback={
                    <ActionButton
                        data-slot="player-captions-button"
                        variant="ghost"
                        isIconOnly
                        icon={captionsIcon}
                        weight={drawn()}
                        {...omit(properties, "xstyle")}
                        action={toggleCaptions}
                        xstyle={[styles.icon, properties.xstyle]}
                    />
                }
            >
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={(attributes) => (
                            <Button
                                isIconOnly
                                {...attributes}
                                data-slot="player-captions-button"
                                variant="ghost"

                                aria-label={locale.render(t.context("label")`Captions`)}
                                xstyle={[styles.icon, properties.xstyle]}
                            />
                        )}
                    >
                        <Icon icon={captionsIcon} weight={drawn()} />
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <DropdownMenuRadioGroup
                            aria-label={locale.render(t.context("label")`Captions`)}
                            value={String(shown())}
                            onValueChange={(place) =>
                                control.showCaptions(place === "-1" ? undefined : Number(place))
                            }
                        >
                            <DropdownMenuRadioItem value="-1">
                                {locale.render(t.context("option")`Off`)}
                            </DropdownMenuRadioItem>
                            <For each={control.captions()}>
                                {(caption, place) => (
                                    <DropdownMenuRadioItem value={String(place())}>
                                        <span lang={caption.language}>{caption.label}</span>
                                    </DropdownMenuRadioItem>
                                )}
                            </For>
                        </DropdownMenuRadioGroup>
                    </DropdownMenuContent>
                </DropdownMenu>
            </Show>
        </Show>
    );
}

/** Render the button that shows a film in a window over others, nothing where there is none. */
export function PlayerPictureInPictureButton(properties: PlayerButtonProperties): JSX.Element {
    const control = usePlayer();

    return (
        <Show when={control.canPictureInPicture()}>
            <ActionButton
                data-slot="player-picture-in-picture-button"
                variant="ghost"
                isIconOnly
                icon={pictureIcon}
                {...omit(properties, "xstyle")}
                action={togglePictureInPicture}
                xstyle={[styles.icon, styles.spare, properties.xstyle]}
            />
        </Show>
    );
}

/** Render the button that shows a film over the whole screen, nothing where a browser cannot. */
export function PlayerFullscreenButton(properties: PlayerButtonProperties): JSX.Element {
    const control = usePlayer();

    return (
        <Show when={control.element() !== undefined && control.canFullscreen()}>
            <ActionButton
                data-slot="player-fullscreen-button"
                variant="ghost"
                isIconOnly
                icon={control.isFullscreen() ? cornersInIcon : cornersOutIcon}
                {...omit(properties, "xstyle")}
                action={toggleFullscreen}
                xstyle={[styles.icon, properties.xstyle]}
            />
        </Show>
    );
}

/** The captions of a film in a language. */
export interface PlayerCaptionTrack {
    /** The address of the captions, as WebVTT. */
    readonly source: string;
    /** The language, as BCP 47 writes it. */
    readonly language: string;
    /** What a person calls them. */
    readonly label: string;
}

/** What plays, and how it is steered from elsewhere. */
export interface PlayerOptions {
    /** The address of what plays. */
    readonly source: string;
    /** What it is called. */
    readonly title?: string;
    /** Who made it. */
    readonly artist?: string;
    /** The address of its picture: a film's before it plays, a sound's cover. */
    readonly picture?: string;
    /** How long it is in seconds, known before it loads. */
    readonly length?: number;
    /** Where it starts, in seconds. */
    readonly start?: number;
    /** Whether it plays as it shows, where the browser lets it. */
    readonly autoplay?: boolean;
    /** Whether it plays again from its end. */
    readonly loop?: boolean;
    /** Its captions, the first shown. */
    readonly captions?: readonly PlayerCaptionTrack[];
    /** Play what comes before it. */
    readonly onPrevious?: () => void;
    /** Play what comes after it. */
    readonly onNext?: () => void;
    /** Handle where it is, as it moves. */
    readonly onTime?: (seconds: number) => void;
    /** Handle its end. */
    readonly onEnded?: () => void;
    /** What steers it elsewhere, a control of its own by default. */
    readonly control?: PlayerControl;
}

/** The properties of what plays, with its element's attributes. */
export type PlayerProperties = PlayerOptions & Omit<PlayerElementProperties, "onEnded">;

/** The names of what only a player reads. */
const OPTIONS = [
    "source",
    "title",
    "artist",
    "picture",
    "length",
    "start",
    "autoplay",
    "loop",
    "captions",
    "onPrevious",
    "onNext",
    "onTime",
    "onEnded",
    "control",
] as const;

/** What plays in the nearest player, null outside one. */
const OptionsContext = createContext<PlayerOptions | null>(null);

/** Read what plays in the nearest player, refusing a part outside one. */
function useOptions(): PlayerOptions {
    const options = useContext(OptionsContext);
    if (options === null) {
        throw new TypeError("a part of a player needs a player around it");
    }

    return options;
}

/** Whether what floats over the nearest film is hidden, never outside a frame. */
const HiddenContext = createContext<Accessor<boolean>>(() => false);

/** Play a film or a sound through the parts inside, offering what a person does as actions. */
export function Player(properties: PlayerProperties): JSX.Element {
    // steer it from elsewhere, or by a control of its own
    const control = untrack(() => properties.control ?? new PlayerControl(properties.length));
    const locale = useLocale();
    const rest = omit(properties, ...OPTIONS);
    const handlers = [
        ...handlersOf(control),
        handle(previousTrack, {
            isEnabled: () => properties.onPrevious !== undefined,
            run: () => properties.onPrevious?.(),
        }),
        handle(nextTrack, {
            isEnabled: () => properties.onNext !== undefined,
            run: () => properties.onNext?.(),
        }),
    ];

    return (
        <PlayerContext value={control}>
            <OptionsContext value={properties}>
                {renderPart(
                    "div",
                    "player",
                    rest,
                    styles.player,
                    mergeProperties(
                        {
                            role: "region" as const,
                            tabindex: -1,
                            get "aria-label"() {
                                return (
                                    properties.title ?? locale.render(t.context("title")`Player`)
                                );
                            },
                        },
                        { ref: handlers },
                    ),
                )}
            </OptionsContext>
        </PlayerContext>
    );
}

/** The properties of a film, with its own attributes, such as `crossorigin` or `preload`. */
export type PlayerVideoProperties = Omit<
    JSX.VideoHTMLAttributes<HTMLVideoElement>,
    "class" | "style" | "children" | "src" | "ref"
> & {
    /** The StyleX styles applied after its own. */
    readonly xstyle?: style.Styles;
};

/** The properties of a sound, with its own attributes, such as `crossorigin` or `preload`. */
export type PlayerAudioProperties = Omit<
    JSX.AudioHTMLAttributes<HTMLAudioElement>,
    "class" | "style" | "children" | "src" | "ref"
>;

/** Make what the element of a player takes. */
function sourceOf(control: PlayerControl, options: PlayerOptions): object {
    // attach it until it goes
    const leaving = new AbortController();
    onCleanup(() => leaving.abort());
    const session = (): PlayerSessionOptions => ({
        title: options.title,
        artist: options.artist,
        picture: options.picture,
        onPrevious: options.onPrevious,
        onNext: options.onNext,
    });

    // keep playing as another takes its place
    let wasPlaying = false;
    createEffect(
        () => options.source,
        (_source, before) => {
            if (before !== undefined && wasPlaying) {
                control.play();
            }
        },
    );

    return merge(
        {
            preload: "metadata",
            playsinline: true,
            ref: (target: HTMLMediaElement) => {
                untrack(() => control.attach(target, leaving.signal));
                untrack(() => control.offer(() => untrack(session), leaving.signal));
            },
            onLoadedMetadata: (event: Event & { readonly currentTarget: HTMLMediaElement }) => {
                const start = untrack(() => options.start);
                if (start !== undefined && event.currentTarget.currentTime === 0) {
                    control.seek(start);
                }
            },
            onTimeUpdate: () => options.onTime?.(untrack(control.time)),
            onPlay: () => (wasPlaying = true),
            onPause: (event: Event & { readonly currentTarget: HTMLMediaElement }) => {
                wasPlaying = event.currentTarget.ended;
            },
            onEnded: () => options.onEnded?.(),
        },
        {
            get src() {
                return options.source;
            },
            get autoplay() {
                return options.autoplay;
            },
            get loop() {
                return options.loop;
            },
        },
    );
}

/** Render the film of a player, with its captions. */
export function PlayerVideo(properties: PlayerVideoProperties): JSX.Element {
    // play it by the element
    const control = usePlayer();
    const options = useOptions();
    const source = sourceOf(control, options);

    return (
        <video
            data-slot="player-video"
            poster={options.picture}
            {...omit(properties, "xstyle")}
            {...source}
            {...style.attrs(styles.film, properties.xstyle)}
        >
            <For each={options.captions ?? []}>
                {(track, place) => (
                    <track
                        kind="captions"
                        src={track.source}
                        srclang={track.language}
                        label={track.label}
                        default={place() === 0 ? true : undefined}
                    />
                )}
            </For>
        </video>
    );
}

/** Render the sound of a player, which shows nothing itself. */
export function PlayerAudio(properties: PlayerAudioProperties): JSX.Element {
    const control = usePlayer();
    const source = sourceOf(control, useOptions());

    return <audio data-slot="player-audio" {...properties} {...source} />;
}

/** The properties of a film's frame, with its element's attributes. */
export type PlayerFrameProperties = PlayerElementProperties & {
    /** Its width divided by its height, 16 to 9 by default. */
    readonly ratio?: number;
};

/** Frame a film, whose bar leaves while it plays and the pointer stands still. */
export function PlayerFrame(properties: PlayerFrameProperties): JSX.Element {
    // follow whether the pointer stands still
    const control = usePlayer();
    const idle = createIdleTimer({ events: [], idleTimeout: IDLE, startManually: true });
    const isAlone = (): boolean => idle.isIdle() && !control.isPaused();
    const isHidden = (): boolean => isAlone() || !isStarted(control);
    let frame: HTMLElement | undefined;

    // show its bar for a while as it starts, and take the focus of the start button, which goes
    createEffect(
        () => isStarted(control),
        (hasStarted, hadStarted) => {
            if (hasStarted && hadStarted !== true) {
                idle.start();
                queueMicrotask(() => {
                    if (document.activeElement === document.body) {
                        frame?.focus({ preventScroll: true });
                    }
                });
            }
        },
    );

    // play it by a click, and show or hide its bar by a tap
    let pointer = "";
    const click = (event: MouseEvent): void => {
        // leave what has its own clicks
        if (!(event.target instanceof HTMLVideoElement)) {
            return;
        }

        // tell a tap from a click
        if (pointer !== "touch") {
            control.toggle();
        } else if (isAlone() || control.isPaused()) {
            idle.start();
        } else {
            idle.triggerIdle();
        }
    };
    const rest = omit(properties, "ratio", "children");

    return renderPart(
        "div",
        "player-frame",
        rest,
        () => [
            styles.frame,
            styles.ratio(properties.ratio ?? 16 / 9),
            control.isFullscreen() && styles.whole,
            isAlone() && styles.alone,
        ],
        mergeProperties(
            {
                tabindex: -1,
                onPointerMove: (event: PointerEvent) => {
                    if (event.pointerType !== "touch") {
                        idle.start();
                    }
                },
                onPointerLeave: (event: PointerEvent) => {
                    // leave a finger that lifts to its tap
                    if (event.pointerType !== "touch") {
                        idle.triggerIdle();
                    }
                },
                onPointerDown: (event: PointerEvent) => (pointer = event.pointerType),
                onKeyDown: (event: KeyboardEvent) => {
                    if (event.key === "Tab") {
                        idle.start();
                    }
                },
                onClick: click,
                onDblClick: (event: MouseEvent) => {
                    const isFilm = event.target instanceof HTMLVideoElement;
                    if (isFilm && pointer !== "touch" && control.canFullscreen()) {
                        void control.toggleFullscreen();
                    }
                },
                get children() {
                    return <HiddenContext value={isHidden}>{properties.children}</HiddenContext>;
                },
            },
            {
                ref: (target: HTMLElement) => {
                    frame = target;
                    control.setFrame(target);
                },
            },
        ),
    );
}

/** Report whether a film has started. */
function isStarted(control: PlayerControl): boolean {
    return control.time() > 0 || !control.isPaused();
}

/** Render the row of a film's parts, which leaves as its frame says. */
export function PlayerBar(properties: PlayerElementProperties): JSX.Element {
    const isHidden = useContext(HiddenContext);

    return renderPart(
        "div",
        "player-bar",
        properties,
        () => [styles.bar, styles.fading, isHidden() && styles.hidden],
        { dir: "ltr" },
    );
}

/** Render what a film's captions say, above its bar. */
export function PlayerCaptions(properties: PlayerElementProperties): JSX.Element {
    const control = usePlayer();
    const isHidden = useContext(HiddenContext);

    return renderPart(
        "div",
        "player-captions",
        properties,
        () => [styles.captions, !isHidden() && styles.raised],
        {
            get children() {
                return (
                    <For each={control.lines()}>
                        {(line) => (
                            <span dir="auto" {...style.attrs(styles.caption)}>
                                {line}
                            </span>
                        )}
                    </For>
                );
            },
        },
    );
}

/** Render the large button that starts a film, nothing once it has started. */
export function PlayerStartButton(properties: PlayerButtonProperties): JSX.Element {
    const control = usePlayer();
    const isShown = (): boolean =>
        !isStarted(control) && !control.isWaiting() && control.failure() === undefined;

    return (
        <Show when={isShown()}>
            <div {...style.attrs(styles.over)}>
                <ActionButton
                    data-slot="player-start-button"
                    variant="outline"
                    isIconOnly
                    size="lg"
                    icon={playIcon}
                    weight="fill"
                    {...omit(properties, "xstyle")}
                    action={togglePlay}
                    xstyle={[styles.start, properties.xstyle]}
                />
            </div>
        </Show>
    );
}

/** Render that a film waits for bytes, nothing while it does not. */
export function PlayerSpinner(): JSX.Element {
    const control = usePlayer();

    return (
        <Show when={control.isWaiting() && control.failure() === undefined}>
            <div data-slot="player-spinner" {...style.attrs(styles.over)}>
                <Spinner />
            </div>
        </Show>
    );
}

/** Render that a film or a sound could not be played, nothing while it can. */
export function PlayerError(properties: PlayerElementProperties): JSX.Element {
    // say so, and offer to try again
    const control = usePlayer();
    const locale = useLocale();

    return (
        <Show when={control.failure()}>
            {renderPart("div", "player-error", properties, [text.sm, styles.failure], {
                role: "alert",
                get children() {
                    return contentOr(properties, () => (
                        <>
                            {locale.render(t`This could not be played.`)}
                            <Button variant="secondary" size="sm" onClick={() => control.retry()}>
                                {locale.render(t.context("action")`Try again`)}
                            </Button>
                        </>
                    ));
                },
            })}
        </Show>
    );
}

/** Render the button that plays what comes before, nothing where nothing does. */
export function PlayerPreviousButton(properties: PlayerButtonProperties): JSX.Element {
    const options = useOptions();

    return (
        <Show when={options.onPrevious !== undefined}>
            <ActionButton
                data-slot="player-previous-button"
                variant="ghost"
                isIconOnly
                weight="fill"
                icon={skipBackIcon}
                {...omit(properties, "xstyle")}
                action={previousTrack}
                xstyle={[styles.icon, properties.xstyle]}
            />
        </Show>
    );
}

/** Render the button that plays what comes after, nothing where nothing does. */
export function PlayerNextButton(properties: PlayerButtonProperties): JSX.Element {
    const options = useOptions();

    return (
        <Show when={options.onNext !== undefined}>
            <ActionButton
                data-slot="player-next-button"
                variant="ghost"
                isIconOnly
                weight="fill"
                icon={skipForwardIcon}
                {...omit(properties, "xstyle")}
                action={nextTrack}
                xstyle={[styles.icon, properties.xstyle]}
            />
        </Show>
    );
}

/** Render the picture of what plays, such as a cover, nothing without one. */
export function PlayerPicture(properties: {
    /** The StyleX styles applied after its own. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    const options = useOptions();

    return (
        <Show when={options.picture}>
            {(source) => (
                <img
                    alt=""
                    data-slot="player-picture"
                    src={source()}
                    {...style.attrs(styles.picture, properties.xstyle)}
                />
            )}
        </Show>
    );
}

/** Render what plays and who made it, nothing without a title. */
export function PlayerTitle(properties: PlayerElementProperties): JSX.Element {
    const options = useOptions();

    return (
        <Show when={options.title}>
            {(title) =>
                renderPart("div", "player-title", properties, [text.sm, styles.line], {
                    get children() {
                        return (
                            <>
                                <span {...style.attrs(styles.title)}>{title()}</span>
                                <Show when={options.artist}>
                                    {(artist) => (
                                        <span {...style.attrs(styles.muted)}>
                                            {" · "}
                                            {artist()}
                                        </span>
                                    )}
                                </Show>
                            </>
                        );
                    },
                })
            }
        </Show>
    );
}

/** The properties of a video player, its element's attributes included. */
export type VideoPlayerProperties = PlayerProperties & {
    /** Its width divided by its height, 16 to 9 by default. */
    readonly ratio?: number;
    /** The parts over the film, the usual ones by default. */
    readonly children?: JSX.Element;
};

/** Render a film with the usual parts over it. */
export function VideoPlayer(properties: VideoPlayerProperties): JSX.Element {
    const rest = omit(properties, "ratio", "children");

    return (
        <Player data-slot="video-player" {...rest}>
            <PlayerFrame ratio={properties.ratio ?? 16 / 9}>
                <PlayerVideo />
                {contentOr(properties, () => (
                    <VideoPlayerParts />
                ))}
            </PlayerFrame>
        </Player>
    );
}

/** Render the usual parts over a film. */
function VideoPlayerParts(): JSX.Element {
    return (
        <>
            <PlayerCaptions />
            <PlayerStartButton />
            <PlayerSpinner />
            <PlayerError xstyle={styles.centered} />
            <PlayerBar>
                <PlayerSeekBar />
                <div {...style.attrs(styles.row)}>
                    <PlayerPreviousButton />
                    <PlayerPlayButton />
                    <PlayerNextButton />
                    <PlayerVolume />
                    <PlayerTime />
                    <span {...style.attrs(styles.space)} />
                    <PlayerCaptionsButton />
                    <PlayerSpeed />
                    <PlayerPictureInPictureButton />
                    <PlayerFullscreenButton />
                </div>
            </PlayerBar>
        </>
    );
}

/** The properties of an audio player, its element's attributes included. */
export type AudioPlayerProperties = PlayerProperties & {
    /** The parts of the sound, the usual ones by default. */
    readonly children?: JSX.Element;
};

/** Render a sound as a row of the usual parts. */
export function AudioPlayer(properties: AudioPlayerProperties): JSX.Element {
    const rest = omit(properties, "children", "xstyle");

    return (
        <Player data-slot="audio-player" {...rest} xstyle={[styles.audio, properties.xstyle]}>
            <PlayerAudio />
            {contentOr(properties, () => (
                <AudioPlayerParts />
            ))}
        </Player>
    );
}

/** Render the usual parts of a sound. */
function AudioPlayerParts(): JSX.Element {
    const control = usePlayer();

    return (
        <>
            <PlayerPicture />
            <div dir="ltr" {...style.attrs(styles.row)}>
                <PlayerPreviousButton />
                <PlayerPlayButton />
                <PlayerNextButton />
            </div>
            <div {...style.attrs(styles.words)}>
                <PlayerTitle />
                <PlayerError />
                <Show when={control.failure() === undefined}>
                    <div dir="ltr" {...style.attrs(styles.row)}>
                        <PlayerSeekBar />
                        <PlayerTime />
                    </div>
                </Show>
            </div>
            <PlayerSpeed />
        </>
    );
}
