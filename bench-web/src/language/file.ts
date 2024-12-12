import { toCamelName } from "@/language/const";
import { makeNode } from "@/language/node";
import { type Transaction } from "@/language/transaction";
import { getCachedHostClient } from "@/proto/services";
import {
  BenchData,
  DownloadFilesResponse_DownloadHandle,
  DriveData,
  EXTENSIONS_BY_FILE_FORMAT,
  FILE_FORMAT_BY_EXTENSION,
  FILE_FORMAT_BY_MIME_TYPE,
  FileData,
  FileFormat,
  FileKind,
  FileRetentionMode,
  FileType,
  IconData,
  MIME_TYPES_BY_FILE_FORMAT,
  NodeReferenceData,
  NodeType,
  ResourceStatus,
  Struct,
  TypeConstraintData,
  UploadFilesResponse_UploadHandle,
} from "@/proto/wire";
import { isNode, isNodeOrRef, makeScope, newNodeId, nodeReference, toNodeRef } from "@/proto/wiring";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE } from "@/ui/icon";
import { AsyncEvent, groupByScalar } from "@/utils/functools";
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

/** A file upload. */
export type FileUpload = {
  status: Ref<FileStatus>;
  isActive: Ref<boolean>;
  bench: BenchData;
  parent: DriveData | null;
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
  isActive: Ref<boolean>;
  nodePtr: NodeReferenceData;
  file: Ref<FileData | null>;
  content: Ref<File | null>;
  getUrl: Ref<string | null>;
  progress: Ref<number>; // [0.0, 100.0]
  includesContent: boolean;
  completion: AsyncEvent;
};

//
// Uploads
// TODO :UX: indicate active uploads in UI (maybe as sticky notification)
//

const uploadsByFileId: Ref<Record<string, FileUpload>> = shallowRef({});
export const fileUploads = computed(() => Object.values(uploadsByFileId.value));
export const activeFileUploads = computed(() => fileUploads.value.filter((u) => u.isActive.value));

/** Extract file info from a native File. Like in bench :ExtractFileInfo */
export async function extractFile(
  content: File,
  identity: NodeReferenceData,
  parent: DriveData | null,
  bench: BenchData,
): Promise<FileData> {
  const title = content.name;

  // guess file type using extension & mime type
  let format: FileFormat | undefined = undefined;
  const mimeType: string | undefined = content.type == "" ? undefined : content.type;
  if (title.includes(".")) {
    const extension = title.split(".").pop();
    if (extension && FILE_FORMAT_BY_EXTENSION[extension.toLowerCase()]) {
      format = FILE_FORMAT_BY_EXTENSION[extension.toLowerCase()];
    }
  }
  if (format == null && mimeType != null && FILE_FORMAT_BY_MIME_TYPE[mimeType]) {
    format = FILE_FORMAT_BY_MIME_TYPE[mimeType];
  }
  let type: FileType;
  if (format == null) {
    type = FileType.GENERIC;
  } else {
    type = Math.floor(format / 10000);
  }

  const file = makeNode({
    metatype: NodeType.FILE,
    id: identity.id,
    parentPtr: parent ? toNodeRef(parent) : undefined,
    benchPtr: toNodeRef(bench),
    region: bench.region,
    status: ResourceStatus.UP,
    kind: FileKind.DRIVE,
    title,
    type,
    mimeType,
    format,
    retention: FileRetentionMode.AUTOMATIC,
    size: BigInt(content.size),
    sha256: await sha256(content),
  });

  // TODO :Incomplete: extract more file metadata :ExtractFileInfo

  // image metadata
  if (type == FileType.IMAGE) {
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
      imageLoaded.resolve();
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
function doUploadFile(
  content: File,
  postUrl: string,
  fields: Record<string, string>,
  onProgress: (progress: number) => void,
): Promise<void> {
  log.trace("file.uploadFile", content.name, humanizeBytes(content.size), { content, postUrl, fields });
  const formData = new FormData();
  for (const [key, value] of Object.entries(fields)) {
    formData.append(key, value);
  }
  formData.append("file", content);

  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", postUrl, true);

    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) {
        onProgress((e.loaded / e.total) * 100);
      }
    };
    xhr.onload = () => {
      if (xhr.status == 200 || xhr.status == 204) {
        resolve();
        onProgress(100);
      } else {
        reject(new Error(`failed to upload file '${content.name}': ${xhr.status} ${xhr.statusText}`));
      }
    };
    xhr.onerror = (e) => {
      reject(new Error(`failed to upload file '${content.name}': ${e}`));
    };
    xhr.send(formData);
  });
}

/** Executes a set of uploads (as parallel as possible). */
async function doUploadFiles(
  txFactory: () => Transaction,
  uploads: FileUpload[],
  options?: { validate: (upload: FileUpload, info: FileData) => void },
): Promise<void> {
  if (uploads.length == 0) return;

  log.trace("file.uploadFiles", uploads);
  // extract files
  for (const upload of uploads) {
    try {
      upload.status.value = FileStatus.PREPARING;
      if (options?.validate) {
        upload.file.value = await extractFile(upload.content, upload.nodePtr, upload.parent, upload.bench);
        options.validate(upload, upload.file.value);
      }
    } catch (e) {
      upload.status.value = FileStatus.FAILED;
      upload.completion.reject(e);
    }
  }

  // get post URLs
  const scope = makeScope({ benchId: uploads[0].bench.id });
  const host = getCachedHostClient(scope);
  let handles: UploadFilesResponse_UploadHandle[] = [];
  try {
    const { response } = await host.uploadFiles({ scope, files: uploads.map((u) => u.file.value!) });
    handles = response.handles;
  } catch (e) {
    log.error("file.uploadFiles.error", uploads, e);
    for (const upload of uploads) {
      upload.status.value = FileStatus.FAILED;
      upload.completion.reject(e);
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
      upload.completion.resolve();
      continue;
    }

    upload.file.value = handle.file; // may have been updated by Host
    upload.getUrl.value = handle.getUrl;

    // actually upload
    try {
      const fields = Struct.toJson(handle.fields!) as Record<string, string>;
      upload.status.value = FileStatus.TRANSFERRING;
      await doUploadFile(upload.content, handle.postUrl, fields, (progress) => {
        upload.progress.value = progress;
      });
      upload.status.value = FileStatus.COMPLETED;
      // actually create File node
      txFactory().create(upload.file.value);
      upload.completion.resolve();
    } catch (e) {
      upload.status.value = FileStatus.FAILED;
      upload.completion.reject(e);
      log.error("file.uploadFile.error", upload, e);
    }
  }
  log.trace("file.uploadFiles.complete", uploads);
}

/** Extracts and uploads the given files to the Host. Returns as soon as the upload starts. */
export function uploadFiles(
  txFactory: () => Transaction,
  contents: File[],
  options: {
    parent?: DriveData;
    bench: BenchData;
    allowedTypes?: FileType[];
    allowedFormats?: FileFormat[];
  },
): FileUpload[] {
  const uploads: FileUpload[] = contents.map((content) => {
    const fileIdentity = nodeReference(NodeType.FILE, newNodeId(), { benchId: options.bench.id });
    const upload: FileUpload = {
      status: shallowRef(FileStatus.PENDING),
      isActive: computed(() => upload.status.value != FileStatus.COMPLETED && upload.status.value != FileStatus.FAILED),
      parent: options?.parent ?? null,
      bench: options.bench,
      nodePtr: fileIdentity,
      file: shallowRef(null),
      content,
      getUrl: shallowRef(null),
      progress: shallowRef(0),
      completion: new AsyncEvent(),
    };
    uploadsByFileId.value[fileIdentity.id!] = upload;
    triggerRef(uploadsByFileId);

    // immediately cache upload as download
    const download = uploadAsDownload(upload);
    cacheDownload(download);

    return markRaw(upload);
  });
  // kick off async
  doUploadFiles(txFactory, uploads, {
    validate: (upload, file) => {
      if (options.allowedTypes && !options.allowedTypes.includes(file.type)) {
        throw new Error(
          `want ${options.allowedTypes.map((t) => toCamelName(FileType, t)).join(" or ")}, got ${toCamelName(FileType, file.type)}`,
        );
      }
      if (options.allowedFormats && (file.format == null || !options.allowedFormats.includes(file.format))) {
        throw new Error(
          `want ${options.allowedFormats.map((f) => toCamelName(FileFormat, f)).join(" or ")}, got ${file.format != null ? toCamelName(FileFormat, file.format) : "unknown"}`,
        );
      }
    },
  });
  return uploads;
}

/** Extracts and upload a file to the Host. Returns as soon as the upload starts. */
export function uploadFile(
  txFactory: () => Transaction,
  content: File,
  options: {
    parent?: DriveData;
    bench: BenchData;
    allowedTypes?: FileType[];
    allowedFormats?: FileFormat[];
  },
): FileUpload {
  const upload = uploadFiles(txFactory, [content], options)[0];
  return upload;
}

//
// Downloads
//

const downloadsByFileId: Ref<Record<string, FileDownload>> = shallowRef({});
export const fileDownloads = computed(() => Object.values(downloadsByFileId.value));
export const activeFileDownloads = computed(() => fileDownloads.value.filter((d) => d.isActive.value));

/** Turn a completed upload into a download. */
function uploadAsDownload(upload: FileUpload): FileDownload {
  const download: FileDownload = {
    status: computed(() => {
      if (upload.status.value == FileStatus.COMPLETED) return FileStatus.COMPLETED;
      else if (upload.status.value == FileStatus.FAILED) return FileStatus.FAILED;
      else return FileStatus.PENDING;
    }),
    isActive: upload.isActive,
    nodePtr: upload.nodePtr,
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
  files: (NodeReferenceData | FileData)[],
  options?: { includeContent?: boolean | ((file: FileData | NodeReferenceData) => boolean) },
): FileDownload[] {
  const downloads = files.map((file) => {
    const download: FileDownload = {
      status: shallowRef(FileStatus.PENDING),
      isActive: computed(
        () => download.status.value != FileStatus.COMPLETED && download.status.value != FileStatus.FAILED,
      ),
      nodePtr: isNode(file, NodeType.FILE) ? toNodeRef(file) : toNodeRef(file),
      file: shallowRef(isNode(file, NodeType.FILE) ? file : null),
      content: shallowRef(null),
      getUrl: shallowRef(null),
      progress: shallowRef(0),
      completion: new AsyncEvent(),
      includesContent:
        typeof options?.includeContent == "function"
          ? options.includeContent(file)
          : (options?.includeContent ?? false),
    };
    cacheDownload(download);
    return markRaw(download);
  });
  pendingDownloads.push(...downloads); // add to pending
  return downloads;
}

/** Download a single file from the Host. Returns as soon as the download starts. */
export function downloadFile(file: NodeReferenceData | FileData, options?: { includeContent?: boolean }): FileDownload {
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
      download.completion.resolve();
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
      download.completion.resolve();
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
    download.completion.resolve();
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

type SomeFile = FileData | NodeReferenceData;

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

const PREFETCH_FILE_TYPES = [FileType.TEXT, FileType.CODE, FileType.IMAGE, FileType.AUDIO, FileType.DOCUMENT];

/** Prefetch the given files (incl. content where it makes sense). */
export async function prefetchFiles(files: FileData[]): Promise<void> {
  downloadFiles(files, {
    includeContent: (file) => {
      if (!isNodeOrRef(file, NodeType.FILE)) return false;
      if (PREFETCH_FILE_TYPES.includes((file as FileData).type)) return true;
      else return false;
    },
  });
}

/** Prefetch the given file. */
export async function prefetchFile(file: FileData): Promise<void> {
  await prefetchFiles([file]);
}

//
// File utilities
//

/** Hash the given file content (SHA-256). */
async function sha256(content: File): Promise<string> {
  const arrayBuffer = await content.arrayBuffer();
  const hashBuffer = await crypto.subtle.digest("SHA-256", arrayBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex;
}

/** Gets the icon for the given file. */
export function getFileIcon(file: FileData): IconData | null {
  if (file.format != null && ICON_BY_FILE_FORMAT[file.format] != null) return ICON_BY_FILE_FORMAT[file.format]!;
  else if (file.type != null && ICON_BY_FILE_TYPE[file.type] != null) return ICON_BY_FILE_TYPE[file.type]!;
  else return null;
}

/** Gets the icon for the given file, if any. */
export function getFileIconMaybe(file: FileData | null): IconData | null {
  if (file == null) return null;
  else return getFileIcon(file);
}

/** Gets the HTML file input accept string for the given file, if any. */
export function getFileAccept(options: {
  allowedTypes?: FileType[];
  allowedFormats?: FileFormat[];
}): string | undefined {
  if (
    (options.allowedTypes == null && options.allowedFormats == null) ||
    options.allowedTypes?.some((f) => f == FileType.GENERIC)
  ) {
    return undefined; // allow all
  }

  // get all allowed formats
  const allowedFormats: FileFormat[] = [];
  if (options.allowedFormats != null) allowedFormats.push(...options.allowedFormats);
  if (options.allowedTypes != null) {
    // iterate over all formats, add those within range of allowed types
    for (const format of Object.values(FILE_FORMAT_BY_EXTENSION)) {
      const type = Math.floor(format / 10000);
      if (options.allowedTypes.includes(type)) allowedFormats.push(format);
    }
  }

  // turn allowed formats into accept string (extensions + mime types)
  const acceptParts: string[] = [];
  for (const format of allowedFormats) {
    const mimeTypes = MIME_TYPES_BY_FILE_FORMAT[format];
    if (mimeTypes != null) {
      mimeTypes.forEach((mimeType) => acceptParts.push(mimeType));
    }
    const extensions = EXTENSIONS_BY_FILE_FORMAT[format];
    if (extensions != null) {
      extensions.forEach((extension) => acceptParts.push(`.${extension}`));
    }
  }

  return acceptParts.join(",");
}

export function getFileAcceptFromConstraint(
  type: FileType,
  constraint: TypeConstraintData | undefined,
): string | undefined {
  if (type == FileType.GENERIC) return undefined;
  return getFileAccept({ allowedTypes: [type] });
}
