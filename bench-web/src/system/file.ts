import { FileData, FileReferenceData, FileType, HostClient, ViewType } from "@/proto/wire";
import { pretendReadonly } from "@/utils/ref";
import { shallowRef, type Ref } from "vue";

/** A file upload. */
export type Upload = {
  file: FileData;
  progress: Ref<number>; // 0-1
};

export type Download = {
  file: FileReferenceData;
  progress: Ref<number>; // 0-1
  includesContent: boolean;
};

const _activeUploads: Ref<Upload[]> = shallowRef([]);
export const activeUploads = pretendReadonly(_activeUploads);

/** Uploads the given files to the Host. */
async function uploadFiles(host: HostClient, files: FileData[], contents: File[]) {
  throw new Error("nocheckin: uploadFile");
}

/** 'Downloads' the given files as get URLs from the Host. */
async function downloadFiles(
  host: HostClient,
  files: (FileReferenceData | FileData)[],
  options?: { includeContent: boolean },
): Promise<string[]> {
  throw new Error("nocheckin: downloadFiles");
}
