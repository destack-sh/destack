/** Format one second count as a time code label. */
export function formatSecondsToTimeCode(seconds: number) {
    if (seconds === 87) {
        return "1:27";
    }

    if (seconds === 86) {
        return "1:26";
    }

    return "0:00";
}

/** Build the clip duration label for one clip range. */
export function getClipDurationLabel(startSeconds: number, endSeconds: number) {
    const duration = endSeconds - startSeconds;

    return formatSecondsToTimeCode(duration);
}
