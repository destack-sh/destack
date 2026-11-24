import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { schema } from "@destack/schema";

/** Play a film or a sound, or pause it. */
export const togglePlay = defineAction({
    name: "player.togglePlay",
    title: t`Play`,
    category: t`Player`,
    keybinding: ["space", "k"],
});

/** Go back ten seconds. */
export const skipBack = defineAction({
    name: "player.skipBack",
    title: t`Back 10 seconds`,
    category: t`Player`,
    keybinding: ["j", "left"],
});

/** Go forward ten seconds. */
export const skipForward = defineAction({
    name: "player.skipForward",
    title: t`Forward 10 seconds`,
    category: t`Player`,
    keybinding: ["l", "right"],
});

/** Go to a time in seconds. */
export const seekTo = defineAction({
    name: "player.seekTo",
    title: t`Go to time`,
    category: t`Player`,
    input: { time: schema.number().nonnegative() },
});

/** Mute the sound, or turn it on again. */
export const toggleMute = defineAction({
    name: "player.toggleMute",
    title: t`Mute`,
    category: t`Player`,
    keybinding: "m",
});

/** Turn the sound up. */
export const volumeUp = defineAction({
    name: "player.volumeUp",
    title: t`Volume up`,
    category: t`Player`,
    keybinding: "up",
});

/** Turn the sound down. */
export const volumeDown = defineAction({
    name: "player.volumeDown",
    title: t`Volume down`,
    category: t`Player`,
    keybinding: "down",
});

/** Show captions, or hide them. */
export const toggleCaptions = defineAction({
    name: "player.toggleCaptions",
    title: t`Captions`,
    category: t`Player`,
    keybinding: "c",
});

/** Show a film over the whole screen, or back in its place. */
export const toggleFullscreen = defineAction({
    name: "player.toggleFullscreen",
    title: t`Full screen`,
    category: t`Player`,
    keybinding: "f",
});

/** Show a film in a window over others, or back in its place. */
export const togglePictureInPicture = defineAction({
    name: "player.togglePictureInPicture",
    title: t`Picture in picture`,
    category: t`Player`,
    keybinding: "p",
});

/** Play what comes before. */
export const previousTrack = defineAction({
    name: "player.previous",
    title: t`Previous`,
    category: t`Player`,
    keybinding: "shift+p",
});

/** Play what comes after. */
export const nextTrack = defineAction({
    name: "player.next",
    title: t`Next`,
    category: t`Player`,
    keybinding: "shift+n",
});
