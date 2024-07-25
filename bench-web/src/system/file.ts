import { getCachedHostClient } from "@/proto/services";
import {
  BlockData,
  FILE_FORMAT_BY_EXTENSION,
  FILE_FORMAT_BY_MIME_TYPE,
  FileData,
  FileFormat,
  FileKind,
  FileReferenceData,
  FileType,
  HostClient,
  NodeType,
  PackageData,
} from "@/proto/wire";
import { isNode, makeScope, toNodeReference } from "@/proto/wiring";
import { makeNode } from "@/system/lang";
import { unpackProtoJson } from "@/system/transaction";
import { AsyncEvent } from "@/utils/functools";
import { log } from "@/utils/log";
import { humanizeBytes } from "@/utils/string";
import { markRaw, ref, shallowRef, type Ref } from "vue";

export enum FileUploadStatus {
  PENDING = 0,
  PREPARING = 1,
  WAITING = 2,
  UPLOADING = 3,
  COMPLETED = 4,
  FAILED = 5,
}

export enum FileDownloadStatus {
  PENDING = 0,
  PREPARING = 1,
  WAITING = 2,
  DOWNLOADING = 3,
  COMPLETED = 4,
  FAILED = 5,
}

/** A file upload. */
export type FileUpload = {
  status: Ref<FileUploadStatus>;
  parent: PackageData | BlockData;
  file: Ref<FileData | null>;
  content: File;
  getUrl: Ref<string | null>;
  progress: Ref<number>; // [0.0, 100.0]
  completion: AsyncEvent;
};

/** A file download. */
export type FileDownload = {
  status: Ref<FileDownloadStatus>;
  filePtr: FileReferenceData;
  file: Ref<FileData | null>;
  content: Ref<File | null>;
  getUrl: Ref<string | null>;
  progress: Ref<number>; // [0.0, 100.0]
  includesContent: boolean;
  completion: AsyncEvent;
};

const _cachedDownloadsByFileId: Record<string, FileDownload> = {};

/** Turn a completed upload into a download. */
function uploadAsDownload(upload: FileUpload): FileDownload {
  if (upload.file.value == null) throw new Error(`missing file for ${upload.content.name}`);
  const download: FileDownload = {
    status: ref(FileDownloadStatus.COMPLETED),
    filePtr: toNodeReference(upload.file.value),
    file: shallowRef(upload.file.value),
    content: shallowRef(upload.content),
    getUrl: upload.getUrl,
    progress: ref(100),
    completion: upload.completion,
  };
  _cachedDownloadsByFileId[upload.file.value.id] = download;
  return download;
}

/** Extract file info from a native File. Like in bench :ExtractFileInfo */
export async function extractFile(content: File, parent: PackageData | BlockData): Promise<FileData> {
  const title = content.name;

  // guess file type using extension & mime type
  let format: FileFormat | undefined = undefined;
  const mimeType: string = content.type;
  if (title.includes(".")) {
    // extension
    const extension = title.split(".").pop();
    if (extension && FILE_FORMAT_BY_EXTENSION[extension.toLowerCase()]) {
      format = FILE_FORMAT_BY_EXTENSION[extension.toLowerCase()];
    }
  }
  if (format == null && FILE_FORMAT_BY_MIME_TYPE[mimeType]) {
    format = FILE_FORMAT_BY_MIME_TYPE[mimeType];
  }
  let coarseType: FileType;
  if (format == null) {
    coarseType = FileType.GENERIC;
  } else {
    coarseType = Math.floor(format / 1000);
  }

  // TODO :Incomplete: extract more file metadata :ExtractFileInfo
  const file = makeNode({
    metatype: NodeType.FILE,
    parentPtr: toNodeReference(parent),
    benchPtr: parent.benchPtr,
    packagePtr: isNode(parent, NodeType.PACKAGE) ? toNodeReference(parent) : parent.packagePtr,
    kind: FileKind.DRIVE,
    title,
    coarseType,
    mimeType,
    format,
    size: BigInt(content.size),
    sha256: await sha256(content),
  });
  return file;
}

/** Actually upload a single file to a presigned post URL. */
async function doUploadFile(content: File, postUrl: string, fields: Record<string, string>): Promise<void> {
  log.trace("file.uploadFile", content.name, humanizeBytes(content.size), { content, postUrl, fields });
  const formData = new FormData();
  for (const [key, value] of Object.entries(fields)) {
    formData.append(key, value);
  }
  formData.append("file", content);
  const response = await fetch(postUrl, {
    method: "POST",
    body: formData,
  });
  if (!response.ok) {
    throw new Error(`failed to upload file '${content.name}': ${response.status} ${response.statusText}`);
  }
  log.trace("file.uploadFile.complete", content.name, humanizeBytes(content.size));
}

/** Executes a set of uploads (as parallel as possible). */
async function doUploadFiles(uploads: FileUpload[]): Promise<void> {
  log.trace("file.uploadFiles", uploads);
  // extract files
  for (const upload of uploads) {
    upload.status.value = FileUploadStatus.PREPARING;
    upload.file.value = await extractFile(upload.content, upload.parent);
  }

  // get post URLs
  const scope = makeScope({ benchId: uploads[0].parent.benchPtr!.id });
  const host = getCachedHostClient(scope);
  const {
    response: { handles },
  } = await host.uploadFiles({ scope, files: uploads.map((u) => u.file.value!) });

  // upload to post URLs
  for (let i = 0; i < uploads.length; i++) {
    const upload = uploads[i];
    const handle = handles[i];
    if (handle.file == null) throw new Error(`missing file from host for ${upload.content.name}`);
    upload.file.value = handle.file; // may have been updated by Host
    upload.getUrl.value = handle.getUrl;
    const fields = unpackProtoJson(handle.fields!) as Record<string, string>;
    
    // actually upload
    upload.status.value = FileUploadStatus.UPLOADING;
    await doUploadFile(upload.content, handle.postUrl, fields);
    upload.status.value = FileUploadStatus.COMPLETED;
    upload.completion.set();
  }
  log.trace("file.uploadFiles.complete", uploads);
}

/** Extracts and uploads the given files to the Host. Returns as soon as the upload starts. */
export function uploadFiles(contents: File[], parent: PackageData | BlockData): FileUpload[] {
  const uploads: FileUpload[] = contents.map((content) => {
    const upload: FileUpload = {
      status: shallowRef(FileUploadStatus.PENDING),
      parent,
      file: shallowRef(null),
      content,
      getUrl: shallowRef(null),
      progress: shallowRef(0),
      completion: new AsyncEvent(),
    };
    return markRaw(upload);
  });
  doUploadFiles(uploads); // kick off async
  return uploads;
}

/** Extracts and upload a file to the Host. Returns as soon as the upload starts. */
export function uploadFile(content: File, parent: PackageData | BlockData): FileUpload {
  const upload = uploadFiles([content], parent)[0];
  return upload;
}

/** 'Downloads' the given files as get URLs from the Host. Returns as soon as the download starts. */
export function downloadFiles(
  host: HostClient,
  files: (FileReferenceData | FileData)[],
  options?: { includeContent: boolean },
): FileDownload[] {
  throw new Error("nocheckin: downloadFiles");
}

/** Hash the given file content (SHA-256). */
async function sha256(content: File): Promise<string> {
  const arrayBuffer = await content.arrayBuffer();
  const hashBuffer = await crypto.subtle.digest("SHA-256", arrayBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex;
}
