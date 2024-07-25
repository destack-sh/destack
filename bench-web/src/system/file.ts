import { FileData, FileReferenceData, FileType, HostClient, ViewType } from "@/proto/wire";
import { pretendReadonly } from "@/utils/ref";
import { shallowRef, type Ref } from "vue";

/** A file upload. */
export type Upload = {
  file: FileData;
  progress: Ref<number>; // [0.0, 100.0]
  completion: Promise<void>;
};

export type Download = {
  file: FileReferenceData;
  progress: Ref<number>; // [0.0, 100.0]
  includesContent: boolean;
  completion: Promise<void>;
};

const _activeUploads: Ref<Upload[]> = shallowRef([]);
export const activeUploads = pretendReadonly(_activeUploads);

/** Extract file info from a native File */
export function extractFileInfo(file: File): FileData {
  throw new Error("nocheckin: extractFileInfo");
}

/** Uploads the given files to the Host. Returns as soon as the upload starts. */
export async function uploadFiles(host: HostClient, files: FileData[], contents: File[]): Promise<Upload[]> {
  throw new Error("nocheckin: uploadFile");
}

/** 'Downloads' the given files as get URLs from the Host. Returns as soon as the download starts. */
export async function downloadFiles(
  host: HostClient,
  files: (FileReferenceData | FileData)[],
  options?: { includeContent: boolean },
): Promise<Download[]> {
  throw new Error("nocheckin: downloadFiles");
}
