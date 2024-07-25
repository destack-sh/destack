import { FileData, FileReferenceData, FileType, HostClient, ViewType } from "@/proto/wire";
import { pretendReadonly } from "@/utils/ref";
import { shallowRef, type Ref } from "vue";



export type Upload = {
  file: FileData;
  progress: Ref<number>; // 0-1
};

export type Download = {
  file: FileReferenceData;
  progress: Ref<number>; // 0-1
};

const _activeUploads: Ref<Upload[]> = shallowRef([]);
export const activeUploads = pretendReadonly(_activeUploads);

async function uploadFiles(host: HostClient, files: FileData[], contents: File[]) {
  throw new Error("nocheckin: uploadFile");
}

async function downloadFiles(host: HostClient, files: FileReferenceData[]) {
  throw new Error("nocheckin: downloadFiles");
}
