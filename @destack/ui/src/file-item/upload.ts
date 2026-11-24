import type { Upload, UploadReason, UploadStatus } from "@destack/object/client";
import { type Accessor, createEffect, createSignal } from "@destack/view";

/** An upload as a file follows it. */
export type FollowedUpload = Pick<Upload, "status" | "reason" | "loaded" | "size" | "watch">;

/** How far a file on its way is. */
export interface UploadProgress {
    /** Whether it is sent, paused, done, failed or cancelled. */
    readonly status: UploadStatus;
    /** Why it failed, absent for one that did not. */
    readonly reason: UploadReason | undefined;
    /** The share of its bytes that arrived, from 0 to 1. */
    readonly share: number;
}

/** Follow how far a file on its way is, absent without one. */
export function createProgress(
    upload: Accessor<FollowedUpload | undefined>,
): Accessor<UploadProgress | undefined> {
    // read it again at each change
    const [progress, setProgress] = createSignal<UploadProgress | undefined>(progressOf(upload()), {
        ownedWrite: true,
    });
    createEffect(upload, (followed) => {
        const read = (): void => void setProgress(progressOf(followed));
        read();

        return followed?.watch(read);
    });

    return progress;
}

/** Read how far a file on its way is. */
function progressOf(upload: FollowedUpload | undefined): UploadProgress | undefined {
    return upload === undefined
        ? undefined
        : {
              status: upload.status,
              reason: upload.reason,
              share: upload.size === 0 ? 1 : upload.loaded / upload.size,
          };
}
