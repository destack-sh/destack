import { swatches } from "@destack/theme/swatch";
import { Hue } from "@destack/color";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createSignal,
    type JSX,
    merge,
    omit,
    onSettled,
    type Setter,
    Show,
    untrack,
    useContext,
    useEnvironment,
} from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    type RenderProperties,
    defineComponent,
} from "@destack/view/declare";
import { t } from "@destack/locale";
import { field } from "@destack/object";

/** The sizes of an avatar. */
const AVATAR_SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
    lg: { title: t.context("option")`Large` },
};

/** The styles of an avatar and its elements. */
const styles = style.create({
    avatar: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        borderRadius: radius.full,
        userSelect: "none",
    },
    image: {
        aspectRatio: "1",
        width: "100%",
        height: "100%",
        borderRadius: radius.full,
        objectFit: "cover",
    },
    loading: {
        display: "none",
    },
    fallback: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: "100%",
        height: "100%",
        borderRadius: radius.full,
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    tinted: (background: string, foreground: string) => ({
        backgroundColor: background,
        color: foreground,
        fontWeight: weight.medium,
    }),
    badge: {
        position: "absolute",
        insetInlineEnd: 0,
        bottom: 0,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "30%",
        height: "30%",
        borderRadius: radius.full,
        backgroundColor: swatch.accentSolid,
        color: swatch.accentLabel,
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}`,
    },
    group: {
        display: "flex",
    },
    grouped: {
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}`,
        marginInlineStart: { default: `calc(-1 * ${space[2]})`, ":first-child": 0 },
    },
    count: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        marginInlineStart: `calc(-1 * ${space[2]})`,
        borderRadius: radius.full,
        backgroundColor: color.muted,
        color: color.mutedForeground,
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}`,
    },
});

/** The width and height of each avatar size: 24, 32 and 40 pixels, after shadcn/ui. */
const sizes = style.create({
    sm: { width: size[1], height: size[1] },
    default: { width: size[2], height: size[2] },
    lg: { width: size[3], height: size[3] },
});

/** Whether an avatar sits in a group, overlapping the one before it. */
const AvatarGroupContext = createContext<AvatarGroupControl | null>(null);

/** The size a group gives the avatars in it and its count. */
export interface AvatarGroupControl {
    /** The size of the group's avatars and count. */
    readonly size: AvatarSize;
}

/** The loading state of the nearest avatar's image, null outside an avatar. */
const AvatarContext = createContext<AvatarControl | null>(null);

/** Whether an avatar's image is loading, shown, or failed and replaced by the fallback. */
export type AvatarImageStatus = "loading" | "loaded" | "error";

/** The loading state an avatar's image reports and its fallback reads. */
export interface AvatarControl {
    /** The status of the image. */
    readonly status: Accessor<AvatarImageStatus>;
    /** Replace the status of the image. */
    readonly setStatus: Setter<AvatarImageStatus>;
    /** The swatch the fallback fills with, the muted colors when absent. */
    readonly color: Accessor<Hue | undefined>;
}

/** The size of an avatar, from smallest to largest. */
export type AvatarSize = keyof typeof AVATAR_SIZES;

/** The properties of an element of an avatar, the native element's attributes included. */
export type AvatarElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The properties of an avatar, the native element's attributes included. */
export type AvatarAttributes = AvatarElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>;

/** The properties of an avatar's image, the native image's attributes included. */
export interface AvatarImageProperties extends AvatarElementProperties<
    JSX.ImgHTMLAttributes<HTMLImageElement>
> {
    /** Report each change of the image's status. */
    readonly onStatusChange?: (status: AvatarImageStatus) => void;
}

/** The properties people change in an avatar's controls and inspector. */
const AVATAR_PROPERTIES = {
    size: field
        .enum(AVATAR_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How large the picture or initials show, as large as its group's when absent.`)
        .optional(),
    color: field
        .enum(Hue.choices)
        .labeled(t.context("label")`Color`)
        .described(t`The hue the initials show on, where there is no picture.`)
        .optional(),
};

/** Render a round picture of a person or organization. */
export const Avatar = defineComponent({
    name: "avatar",
    properties: AVATAR_PROPERTIES,
    render(properties: RenderProperties<AvatarAttributes, typeof AVATAR_PROPERTIES>): JSX.Element {
        // overlap the avatar in a group at the group's size, and start with its image loading
        const group = useContext(AvatarGroupContext);
        const avatar = merge(group ?? {}, properties);
        const shown = (): AvatarSize => avatar.size ?? "default";
        const rest = omit(avatar, "size", "color");
        const isGrouped = group !== null;
        const [status, setStatus] = createSignal<AvatarImageStatus>("loading", {
            ownedWrite: true,
        });

        return (
            <AvatarContext value={{ status, setStatus, color: () => avatar.color }}>
                {renderPart(
                    "span",
                    "avatar",
                    rest,
                    () => [styles.avatar, sizes[shown()], isGrouped && styles.grouped],
                    {
                        get "data-size"() {
                            return shown();
                        },
                        get "data-color"() {
                            return avatar.color;
                        },
                    },
                )}
            </AvatarContext>
        );
    },
});

/** Render an avatar's image, hidden until it loads. */
export function AvatarImage(properties: AvatarImageProperties): JSX.Element {
    // read the avatar the image reports to
    const avatar = useAvatar();
    const rest = omit(properties, "onStatusChange");

    // report the image's status to the avatar and its owner
    const report = (status: AvatarImageStatus): void => {
        avatar.setStatus(status);
        properties.onStatusChange?.(status);
    };

    return (
        <Show when={avatar.status() !== "error"}>
            {renderPart(
                "img",
                "avatar-image",
                rest,
                () => [styles.image, avatar.status() === "loading" && styles.loading],
                { onLoad: () => report("loaded"), onError: () => report("error") },
            )}
        </Show>
    );
}

/** The properties of an avatar's fallback, the native element's attributes included. */
export type AvatarFallbackProperties = AvatarElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> & {
    /** The milliseconds before it shows while an image loads, at once by default. */
    readonly delay?: number;
};

/** Render what an avatar shows while its image loads or after it failed, such as initials. */
export function AvatarFallback(properties: AvatarFallbackProperties): JSX.Element {
    // wait for a picture that loads at once, so the fallback does not flash before it
    const avatar = useAvatar();
    const { clock } = useEnvironment();
    const delay = untrack(() => properties.delay ?? 0);
    const [isDue, setDue] = createSignal(delay === 0, { ownedWrite: true });
    const rest = omit(properties, "delay");
    onSettled(() => {
        // wait only where there is a delay
        if (delay === 0) {
            return undefined;
        }
        const timer = clock.setTimeout(() => void setDue(true), delay);

        return () => clock.clearTimeout(timer);
    });

    return (
        <Show when={avatar.status() === "error" || (avatar.status() === "loading" && isDue())}>
            {renderPart("span", "avatar-fallback", rest, () => {
                // tint the fallback in its color, as a badge tints itself
                const tint = avatar.color();

                return [
                    text.xs,
                    state.framed,
                    styles.fallback,
                    tint !== undefined && styles.tinted(swatches[tint].solid, swatches[tint].label),
                ];
            })}
        </Show>
    );
}

/** Render a status dot or icon on an avatar's corner. */
export function AvatarBadge(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "avatar-badge", properties, styles.badge);
}

/** The properties of an avatar group, the native element's attributes included. */
export type AvatarGroupAttributes = AvatarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>;

/** The properties people change in an avatar group's controls and inspector. */
const AVATAR_GROUP_PROPERTIES = {
    size: field
        .enum(AVATAR_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How large each picture or initials show.`)
        .default("default"),
};

/** Render avatars overlapping in a row, at one size. */
export const AvatarGroup = defineComponent({
    name: "avatarGroup",
    properties: AVATAR_GROUP_PROPERTIES,
    render(
        properties: RenderProperties<AvatarGroupAttributes, typeof AVATAR_GROUP_PROPERTIES>,
    ): JSX.Element {
        return (
            <AvatarGroupContext
                value={{
                    get size() {
                        return properties.size;
                    },
                }}
            >
                {renderPart("div", "avatar-group", omit(properties, "size"), styles.group)}
            </AvatarGroupContext>
        );
    },
});

/** Render the count of the avatars a group leaves out. */
export function AvatarGroupCount(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // size the count as the group's avatars
    const group = useAvatarGroup();

    return renderPart(
        "div",
        "avatar-group-count",
        properties,
        () => [text.xs, state.framed, styles.count, sizes[group.size]],
        {
            get "data-size"() {
                return group.size;
            },
        },
    );
}

/** Read the nearest avatar group, refusing a part outside one. */
export function useAvatarGroup(): AvatarGroupControl {
    const group = useContext(AvatarGroupContext);
    if (group === null) {
        throw new TypeError("an avatar group count needs an avatar group around it");
    }

    return group;
}

/** Read the nearest avatar, refusing a part outside one. */
export function useAvatar(): AvatarControl {
    const avatar = useContext(AvatarContext);
    if (avatar === null) {
        throw new TypeError("avatar elements need an avatar around them");
    }

    return avatar;
}

/** The properties of an avatar, its native attributes included. */
export type AvatarProperties = ComponentProperties<typeof Avatar>;

/** The properties of an avatar group, its native attributes included. */
export type AvatarGroupProperties = ComponentProperties<typeof AvatarGroup>;
