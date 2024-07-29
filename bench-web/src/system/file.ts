import { getCachedHostClient } from "@/proto/services";
import {
  BlockData,
  DownloadFilesResponse_DownloadHandle,
  FILE_FORMAT_BY_EXTENSION,
  FILE_FORMAT_BY_MIME_TYPE,
  FileData,
  FileFormat,
  FileKind,
  FileReferenceData,
  FileType,
  IconData,
  NodeReferenceData,
  NodeType,
  PackageData,
  StructType,
  UploadFilesResponse_UploadHandle,
} from "@/proto/wire";
import { isNode, isStruct, makeScope, newNodeId, nodeReference, toNodeReference } from "@/proto/wiring";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE, makeIcon } from "@/system/icon";
import { makeNode, toCamelName } from "@/system/lang";
import { unpackProtoJson, type Transaction } from "@/system/transaction";
import { AsyncEvent, groupByScalar, onEveryTick } from "@/utils/functools";
import { log } from "@/utils/log";
import { humanizeBytes } from "@/utils/string";
import { computed, markRaw, shallowRef, toRef, triggerRef, watch, type MaybeRef, type Ref } from "vue";

export enum FileStatus {
  PENDING = 0,
  PREPARING = 1,
  TRANSFERRING = 3,
  COMPLETED = 4,
  FAILED = 5,
}

const ICON_BY_FILE_STATUS: Record<FileStatus, IconData> = {
  [FileStatus.PENDING]: makeIcon({ faName: "fas fa-hourglass-half" }),
  [FileStatus.PREPARING]: makeIcon({ faName: "fas fa-circle-notch" }),
  [FileStatus.TRANSFERRING]: makeIcon({ faName: "fas fa-circle-notch" }),
  [FileStatus.COMPLETED]: makeIcon({ faName: "fas fa-check" }),
  [FileStatus.FAILED]: makeIcon({ faName: "fas fa-exclamation-triangle" }),
};

export function getFileStatusName(status: FileStatus): string {
  return toCamelName(FileStatus, status);
}

export function getFileStatusIcon(status: FileStatus): IconData {
  return ICON_BY_FILE_STATUS[status];
}

/** A file upload. */
export type FileUpload = {
  status: Ref<FileStatus>;
  parent: PackageData | BlockData;
  nodePtr: NodeReferenceData;
  file: Ref<FileData | null>;
  content: File;
  getUrl: Ref<string | null>;
  progress: Ref<number>; // [0.0, 100.0]
  completion: AsyncEvent;
};

/** A file download. */
export type FileDownload = {
  status: Ref<FileStatus>;
  nodePtr: NodeReferenceData;
  filePtr: FileReferenceData | null;
  file: Ref<FileData | null>;
  content: Ref<File | null>;
  getUrl: Ref<string | null>;
  progress: Ref<number>; // [0.0, 100.0]
  includesContent: boolean;
  completion: AsyncEvent;
};

//
// Uploads
//

const uploadsByFileId: Ref<Record<string, FileUpload>> = shallowRef({}); // nocheckin: track uploads

/** Extract file info from a native File. Like in bench :ExtractFileInfo */
export async function extractFile(
  content: File,
  identity: NodeReferenceData,
  parent: PackageData | BlockData,
): Promise<FileData> {
  const title = content.name;

  // guess file type using extension & mime type
  let format: FileFormat | undefined = undefined;
  const mimeType: string = content.type;
  if (title.includes(".")) {
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

  const file = makeNode({
    metatype: NodeType.FILE,
    id: identity.id,
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

  // TODO :Incomplete: extract more file metadata :ExtractFileInfo

  // image metadata
  if (coarseType == FileType.IMAGE) {
    // turn into data URL & load as Image (this feels a bit hacky)
    const imageLoaded = new AsyncEvent();
    const contentAsDataUrl = window.URL.createObjectURL(content);
    const image = new Image();
    image.src = contentAsDataUrl;
    image.onload = () => {
      file.width = image.width;
      file.height = image.height;
      file.aspectRatio = file.width / file.height;
      window.URL.revokeObjectURL(contentAsDataUrl);
      imageLoaded.set();
    };
    image.onerror = (e) => {
      window.URL.revokeObjectURL(contentAsDataUrl);
      imageLoaded.reject(e);
    };
    await imageLoaded.wait();
  }

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
  const response = await fetch(postUrl, { method: "POST", body: formData });
  if (!response.ok) {
    throw new Error(`failed to upload file '${content.name}': ${response.status} ${response.statusText}`);
  }
  log.trace("file.uploadFile.complete", content.name, humanizeBytes(content.size));
}

/** Executes a set of uploads (as parallel as possible). */
async function doUploadFiles(tx: Transaction, uploads: FileUpload[]): Promise<void> {
  log.trace("file.uploadFiles", uploads);
  // extract files
  for (const upload of uploads) {
    upload.status.value = FileStatus.PREPARING;
    upload.file.value = await extractFile(upload.content, upload.nodePtr, upload.parent);
  }

  // get post URLs
  const scope = makeScope({ benchId: uploads[0].parent.benchPtr!.id });
  const host = getCachedHostClient(scope);
  let handles: UploadFilesResponse_UploadHandle[] = [];
  try {
    const { response } = await host.uploadFiles({ scope, files: uploads.map((u) => u.file.value!) });
    handles = response.handles;
  } catch (e) {
    log.error("file.uploadFiles.error", uploads, e);
    for (const upload of uploads) {
      upload.status.value = FileStatus.FAILED;
      upload.completion.set();
    }
    return;
  }

  const handlesById = groupByScalar(handles, (h) => h.file!.id);
  // upload to post URLs
  for (const upload of uploads) {
    const handle = handlesById[upload.file.value!.id];
    if (handle == null || handle.file == null) {
      // couldn't get upload handle for file
      upload.status.value = FileStatus.FAILED;
      upload.completion.set();
      continue;
    }

    upload.file.value = handle.file; // may have been updated by Host
    upload.getUrl.value = handle.getUrl;

    // actually upload
    try {
      const fields = unpackProtoJson(handle.fields!) as Record<string, string>;
      upload.status.value = FileStatus.TRANSFERRING;
      await doUploadFile(upload.content, handle.postUrl, fields);
      upload.status.value = FileStatus.COMPLETED;

      // actually create File node
      tx.create(upload.file.value);
    } catch (e) {
      upload.status.value = FileStatus.FAILED;
      log.error("file.uploadFile.error", upload, e);
    }
    upload.completion.set();
  }
  log.trace("file.uploadFiles.complete", uploads);
}

/** Extracts and uploads the given files to the Host. Returns as soon as the upload starts. */
export function uploadFiles(tx: Transaction, contents: File[], parent: PackageData | BlockData): FileUpload[] {
  const uploads: FileUpload[] = contents.map((content) => {
    const fileIdentity = nodeReference(NodeType.FILE, newNodeId(), { benchId: parent.benchPtr!.id });
    const upload: FileUpload = {
      status: shallowRef(FileStatus.PENDING),
      parent,
      nodePtr: fileIdentity,
      file: shallowRef(null),
      content,
      getUrl: shallowRef(null),
      progress: shallowRef(0),
      completion: new AsyncEvent(),
    };

    // immediately cache upload as download
    const download = uploadAsDownload(upload);
    cacheDownload(download);

    return markRaw(upload);
  });
  doUploadFiles(tx, uploads); // kick off async
  return uploads;
}

/** Extracts and upload a file to the Host. Returns as soon as the upload starts. */
export function uploadFile(tx: Transaction, content: File, parent: PackageData | BlockData): FileUpload {
  const upload = uploadFiles(tx, [content], parent)[0];
  return upload;
}

//
// Downloads
//

// NOTE :Performance: persist downloads cache in local storage?
const downloadsByFileId: Ref<Record<string, FileDownload>> = shallowRef({});

/** Turn a completed upload into a download. */
function uploadAsDownload(upload: FileUpload): FileDownload {
  const download: FileDownload = {
    status: computed(() => {
      if (upload.status.value == FileStatus.COMPLETED) return FileStatus.COMPLETED;
      else if (upload.status.value == FileStatus.FAILED) return FileStatus.FAILED;
      else return FileStatus.PENDING;
    }),
    nodePtr: upload.nodePtr,
    filePtr: null, // :RichReferences
    file: upload.file,
    content: shallowRef(upload.content),
    getUrl: upload.getUrl,
    progress: upload.progress,
    completion: upload.completion,
    includesContent: true,
  };
  return markRaw(download);
}

function cacheDownload(download: FileDownload) {
  downloadsByFileId.value[download.nodePtr.id!] = download;
  triggerRef(downloadsByFileId);
}

const FILE_DOWNLOAD_BATCH_INTERVAL = 40; // ms
const pendingDownloads: FileDownload[] = [];

// periodically batch downloads
setInterval(() => {
  if (pendingDownloads.length > 0) {
    const toDownload = pendingDownloads.splice(0, pendingDownloads.length);
    doDownloadFiles(toDownload);
  }
}, FILE_DOWNLOAD_BATCH_INTERVAL);

/**
 * 'Downloads' the given files as get URLs from the Host. Returns as soon as the download starts.
 * All pending downloads are batched and execute at some point in the future in parallel.
 */
export function downloadFiles(
  files: (FileReferenceData | NodeReferenceData | FileData)[],
  options?: { includeContent?: boolean },
): FileDownload[] {
  const downloads = files.map((file) => {
    const download: FileDownload = {
      status: shallowRef(FileStatus.PENDING),
      nodePtr: isNode(file, NodeType.FILE) ? toNodeReference(file) : file,
      filePtr: isStruct(file, StructType.FILE_REFERENCE) ? file : null, // :RichReferences
      file: shallowRef(isNode(file, NodeType.FILE) ? file : null),
      content: shallowRef(null),
      getUrl: shallowRef(null),
      progress: shallowRef(0),
      completion: new AsyncEvent(),
      includesContent: options?.includeContent ?? false,
    };
    cacheDownload(download);
    return markRaw(download);
  });
  pendingDownloads.push(...downloads); // add to pending
  return downloads;
}

/** Download a single file from the Host. Returns as soon as the download starts. */
export function downloadFile(
  file: FileReferenceData | NodeReferenceData | FileData,
  options?: { includeContent?: boolean },
): FileDownload {
  const download = downloadFiles([file], options)[0];
  return download;
}

/** Executes a set of downloads (as parallel as possible). */
async function doDownloadFiles(downloads: FileDownload[]): Promise<void> {
  log.trace("file.downloadFiles", downloads);

  // get get URLs
  const scope = makeScope({ benchId: downloads[0].nodePtr.benchId });
  const host = getCachedHostClient(scope);
  let handles: DownloadFilesResponse_DownloadHandle[] = [];
  try {
    const { response } = await host.downloadFiles({ scope, files: downloads.map((d) => d.nodePtr) });
    handles = response.handles;
  } catch (e) {
    log.error("file.downloadFiles.error", downloads, e);
    for (const download of downloads) {
      download.status.value = FileStatus.FAILED;
      download.completion.set();
    }
    return;
  }
  const handlesById = groupByScalar(handles, (h) => h.file!.id);

  // download from get URLs (if content is included)
  for (const download of downloads) {
    download.status.value = FileStatus.PREPARING;
    const handle = handlesById[download.nodePtr.id!];
    if (handle == null || handle.file == null) {
      // couldn't get download handle for file
      download.status.value = FileStatus.FAILED;
      download.completion.set();
      continue;
    }

    download.file.value = handle.file;
    download.getUrl.value = handle.getUrl;

    // download content
    if (download.includesContent) {
      try {
        download.status.value = FileStatus.TRANSFERRING;
        download.content.value = await doDownloadFile(download.getUrl.value, download.file.value);
        download.status.value = FileStatus.COMPLETED;
      } catch (e) {
        download.status.value = FileStatus.FAILED;
        log.error("file.downloadFile.error", download, e);
      }
    } else {
      download.status.value = FileStatus.COMPLETED;
    }
    download.completion.set();
  }
}

/** Actually download file content from the given URL. */
async function doDownloadFile(getUrl: string, file: FileData): Promise<File> {
  const response = await fetch(getUrl);
  if (!response.ok) {
    throw new Error(`failed to download file from ${getUrl}: ${response.status} ${response.statusText}`);
  }
  const content = await response.blob();
  return new File([content], file.title, { type: file.mimeType });
}

type SomeFile = FileData | FileReferenceData | NodeReferenceData;

/** Gets the existing download for the given file. */
export function getCachedFileDownload(file: SomeFile): FileDownload | null {
  return downloadsByFileId.value[file.id!];
}

/** Gets or creates a download for the given file as a ref. */
export function useFileDownload(
  file: MaybeRef<SomeFile | null | undefined>,
  options?: { includeContent?: boolean },
): Ref<FileDownload | null> {
  const fileRef = toRef(file) as Ref<SomeFile | null>;
  const download: Ref<FileDownload | null> = shallowRef(null);

  watch(
    fileRef,
    (newFile) => {
      if (newFile == null) {
        download.value = null;
      } else {
        const existing = getCachedFileDownload(newFile);
        if (existing != null) {
          download.value = existing;
        } else {
          download.value = downloadFile(newFile, options);
        }
      }
    },
    { immediate: true },
  );

  return download;
}

/** Hash the given file content (SHA-256). */
async function sha256(content: File): Promise<string> {
  const arrayBuffer = await content.arrayBuffer();
  const hashBuffer = await crypto.subtle.digest("SHA-256", arrayBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex;
}

/** Gets the icon for the given file. */
export function getFileIcon(file: FileData | FileReferenceData): IconData | null {
  if (file.format != null && ICON_BY_FILE_FORMAT[file.format] != null) return ICON_BY_FILE_FORMAT[file.format]!;
  else if (file.coarseType != null && ICON_BY_FILE_TYPE[file.coarseType] != null)
    return ICON_BY_FILE_TYPE[file.coarseType]!;
  else return null;
}

/** Gets the icon for the given file, if any. */
export function getFileIconMaybe(file: FileData | FileReferenceData | null): IconData | null {
  if (file == null) return null;
  else return getFileIcon(file);
}
