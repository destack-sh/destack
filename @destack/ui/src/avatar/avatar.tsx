import * as style from "@destack/style";
import { color, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createSignal,
    type JSX,
    merge,
    omit,
    type Setter,
    Show,
    useContext,
} from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The size of an avatar that sets none. */
const DEFAULTS: Required<Pick<AvatarProperties, "size">> = { size: "default" };

/** The styles of an avatar and its elements. */
const styles = style.create({
    avatar: {
        position: "relative",
        display: "flex",
        flexShrink: 0,
        borderRadius: "50%",
        userSelect: "none",
    },
    image: {
        aspectRatio: "1",
        width: "100%",
        height: "100%",
        borderRadius: "50%",
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
        borderRadius: "50%",
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    badge: {
        position: "absolute",
        insetInlineEnd: 0,
        bottom: 0,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "30%",
        height: "30%",
        borderRadius: "50%",
        backgroundColor: color.primary,
        color: color.primaryForeground,
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
        width: size[3],
        height: size[3],
        marginInlineStart: `calc(-1 * ${space[2]})`,
        borderRadius: "50%",
        backgroundColor: color.muted,
        color: color.mutedForeground,
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}`,
    },
});

/** The width and height of each avatar size. */
const sizes = style.create({
    sm: { width: size[2], height: size[2] },
    default: { width: size[3], height: size[3] },
    lg: { width: size[4], height: size[4] },
});

/** Whether an avatar sits in a group, overlapping the one before it. */
const AvatarGroupContext = createContext(false);

/** The loading state of the nearest avatar's image, null outside an avatar. */
const AvatarContext = createContext<AvatarImageState | null>(null);

/** Whether an avatar's image is loading, shown, or failed and replaced by the fallback. */
export type AvatarImageStatus = "loading" | "loaded" | "error";

/** The loading state an avatar's image reports and its fallback reads. */
interface AvatarImageState {
    /** The status of the image. */
    readonly status: Accessor<AvatarImageStatus>;
    /** Replace the status of the image. */
    readonly setStatus: Setter<AvatarImageStatus>;
}

/** The size of an avatar, from smallest to largest. */
export type AvatarSize = "sm" | "default" | "lg";

/** The properties of an element of an avatar, the native element's attributes included. */
export type AvatarElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The properties of an avatar, the native element's attributes included. */
export interface AvatarProperties extends AvatarElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> {
    /** The width and height, default by default. */
    readonly size?: AvatarSize;
}

/** The properties of an avatar's image, the native image's attributes included. */
export interface AvatarImageProperties extends AvatarElementProperties<
    JSX.ImgHTMLAttributes<HTMLImageElement>
> {
    /** Report each change of the image's status. */
    readonly onStatusChange?: (status: AvatarImageStatus) => void;
}

/** Render a round picture of a person or organisation, showing its fallback until the image loads. */
export function Avatar(properties: AvatarProperties): JSX.Element {
    // overlap the avatar in a group and start with its image loading
    const avatar = merge(DEFAULTS, properties);
    const rest = omit(avatar, "size", "xstyle", "style");
    const isGrouped = useContext(AvatarGroupContext);
    const [status, setStatus] = createSignal<AvatarImageStatus>("loading", { ownedWrite: true });

    return (
        <AvatarContext value={{ status, setStatus }}>
            <span
                data-slot="avatar"
                data-size={avatar.size}
                {...rest}
                {...style.attributes(
                    [styles.avatar, sizes[avatar.size], isGrouped && styles.grouped, avatar.xstyle],
                    avatar.style,
                )}
            />
        </AvatarContext>
    );
}

/** Render an avatar's image, hidden until it loads. */
export function AvatarImage(properties: AvatarImageProperties): JSX.Element {
    // read the avatar the image reports to
    const state = useAvatar();
    const rest = omit(properties, "xstyle", "style", "onStatusChange");

    // report the image's status to the avatar and its owner
    const report = (status: AvatarImageStatus): void => {
        state.setStatus(status);
        properties.onStatusChange?.(status);
    };

    return (
        <Show when={state.status() !== "error"}>
            <img
                data-slot="avatar-image"
                {...rest}
                onLoad={() => report("loaded")}
                onError={() => report("error")}
                {...style.attributes(
                    [
                        styles.image,
                        state.status() === "loading" && styles.loading,
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            />
        </Show>
    );
}

/** Render what an avatar shows while its image loads or after it failed, such as initials. */
export function AvatarFallback(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const state = useAvatar();

    return (
        <Show when={state.status() !== "loaded"}>
            {renderPart("span", "avatar-fallback", properties, [text.footnote, styles.fallback])}
        </Show>
    );
}

/** Render a status dot or icon on an avatar's corner. */
export function AvatarBadge(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "avatar-badge", properties, styles.badge);
}

/** Render avatars overlapping in a row. */
export function AvatarGroup(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return (
        <AvatarGroupContext value={true}>
            {renderPart("div", "avatar-group", properties, styles.group)}
        </AvatarGroupContext>
    );
}

/** Render the count of the avatars a group leaves out. */
export function AvatarGroupCount(
    properties: AvatarElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "avatar-group-count", properties, [text.footnote, styles.count]);
}

/** Read the image state of the nearest avatar. */
function useAvatar(): AvatarImageState {
    const state = useContext(AvatarContext);
    if (state === null) {
        throw new TypeError("avatar elements need an avatar around them");
    }

    return state;
}
