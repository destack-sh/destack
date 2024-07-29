<script lang="ts" setup>
import { FileFormat, FileReferenceData, FileType, NodeType, ViewData } from "@/proto/wire";
import { toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import {
  FileStatus,
  getFileIconMaybe,
  getFileStatusIcon,
  getFileStatusName,
  uploadFile,
  useFileDownload,
  type FileUpload,
} from "@/system/file";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE, IconInline } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { canvas, pkg, pkgConnection } from "@/system/space";
import { FILE_TYPE_BY_VIEW_TYPE } from "@/system/view";
import { useDropZone } from "@/utils/drag";
import { humanizeBytes } from "@/utils/string";
import { makeViewId, ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: FileReferenceData } & Partial<
    Pick<
      ViewData,
      | "type"
      | "name"
      | "title"
      | "text"
      | "icon"
      | "valueType"
      | "nodePtr"
      | "variant"
      | "isInput"
      | "isInline"
      | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fileType = computed(() => {
  if (props.valueType?.constraint?.fileType != null) return props.valueType.constraint.fileType;
  else if (props.type != null) return FILE_TYPE_BY_VIEW_TYPE[props.type] ?? FileType.GENERIC;
  else return FileType.GENERIC;
});
const fileFormat = computed(() => props.valueType?.constraint?.fileFormat);
const facetIcon = computed(() => {
  if (fileFormat.value != null && ICON_BY_FILE_FORMAT[fileFormat.value] != null) {
    return ICON_BY_FILE_FORMAT[fileFormat.value];
  } else {
    return ICON_BY_FILE_TYPE[fileType.value];
  }
});
const facetName = computed(() => {
  if (fileFormat.value != null) {
    return `${toCamelName(FileFormat, fileFormat.value)} ${toCamelName(FileType, fileType.value)}`;
  } else {
    return toCamelName(FileType, fileType.value);
  }
});

//
// Interaction
//

const fileInputRef = ref<HTMLInputElement | null>(null);
const containerRef = ref<HTMLElement | null>(null);
const upload: Ref<FileUpload | null> = ref(null);
const download = useFileDownload(toRef(props, "modelValue"));
const optimisticValue = computed(() => upload.value?.file.value ?? download.value?.file.value ?? props.modelValue);

async function onFileSelected(files: File[]) {
  if (files.length == 0) return;
  if (pkg.value == null) throw new Error("no current package");
  const content = files[0];
  // NOTE :Incomplete: uploaded file should be attributed to closest ancestor block, not package (?)
  try {
    upload.value = uploadFile(() => pkgConnection.tx, content, pkg.value);
    await upload.value.completion.wait();
    if (upload.value.file.value == null) throw new Error("missing file in upload");
    emit("update:modelValue", toNodeReference(upload.value.file.value));
  } finally {
    upload.value = null;
  }
}

const { isInDropZone } = useDropZone({
  name: "file",
  container: containerRef,
  kinds: ["file"],
  onDrop: (dragged, event) => {
    if (dragged.kind == "file") {
      if (event.dataTransfer?.files.length == 0) return;
      onFileSelected(Array.from(event.dataTransfer!.files));
    }
  },
  isEnabled: computed(() => props.isInput && !props.isDisabled),
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- Actual file input (hidden) -->
    <!-- nocheckin: constrain file types -->
    <input
      ref="fileInputRef"
      type="file"
      class="hidden"
      @change="
        (e) => {
          onFileSelected(Array.from((e.target as HTMLInputElement).files!));
          (e.target as HTMLInputElement).value = ''; // clear value
        }
      "
    />

    <!-- Dropdown -->
    <button
      v-if="!isInline"
      ref="containerRef"
      class="group flex w-full flex-row items-center rounded border px-2.5 py-1 transition-all duration-75 data-[popover=true]:border-gray-300"
      :class="[
        isInDropZone
          ? 'border-primary-400 bg-primary-200 outline outline-1 outline-primary-400'
          : 'border-gray-200  hover:border-gray-300',
      ]"
      @click="fileInputRef!.click()"
    >
      <!-- Current value -->
      <span v-if="optimisticValue != null">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5 text-gray-700" />
        <a class="decoration-gray-300 underline-offset-3 hover:underline hover:decoration-primary-900">
          {{ optimisticValue.title ?? "???" }}
        </a>
        <span class="ml-1.5 text-xs text-gray-400">{{ humanizeBytes(17000) }}</span>
        <i
          v-if="upload != null && upload.isActive.value"
          class="fas fa-spinner-third ml-1.5 animate-spin text-gray-400"
        />
      </span>
      <span
        v-else
        class="select-none transition-colors duration-75"
        :class="isInDropZone ? 'text-primary-900' : 'text-gray-400'"
      >
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
      <!-- Controls -->
      <div v-if="!props.isDisabled && props.isInput" class="ml-auto flex-shrink-0 pl-1.5">
        <!-- Clear -->
        <i
          v-if="modelValue != null && !valueType?.isRequired"
          role="button"
          class="fas fa-xmark mr-2 text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        />
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-primary-900" />
      </div>
    </button>

    <!-- Inline drop area -->
    <div
      v-else-if="optimisticValue == null"
      ref="containerRef"
      class="flex h-full min-h-[80px] w-full cursor-pointer flex-col justify-center rounded border px-2.5 py-1 text-center transition-all duration-75"
      :class="[
        isInDropZone
          ? 'border-primary-400 bg-primary-200 text-primary-900 outline outline-1 outline-primary-400'
          : ' border-gray-200 text-gray-400  hover:border-gray-300',
      ]"
      @click="fileInputRef!.click()"
    >
      <span class="select-none transition-colors duration-75">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
    </div>

    <!-- Inline value -->
    <div
      v-else
      ref="containerRef"
      class="group/inline relative flex h-full min-h-[80px] w-full flex-col justify-center rounded border border-gray-200"
      :class="[isInDropZone ? 'border-primary-400 outline outline-1 outline-primary-400' : '']"
    >
      <!-- NOTE :Incomplete: proper file content views (image with proper size & thumbnail, audio, ...) -->
      <!-- Image File -->
      <div v-if="optimisticValue?.coarseType == FileType.IMAGE && download?.getUrl.value != null">
        <img
          :key="download.getUrl.value"
          :src="download.getUrl.value"
          class="h-full w-full rounded"
          :alt="optimisticValue?.title ?? '???'"
        />
      </div>
      <!-- Generic File -->
      <div v-else-if="optimisticValue != null" class="flex h-full w-full justify-center text-center">
        <span>
          <IconInline v-bind="getFileIconMaybe(optimisticValue) ?? facetIcon" class="text-gray-700" />
          <a class="ml-1.5 decoration-gray-300 underline-offset-3 hover:underline hover:decoration-primary-900">
            {{ optimisticValue?.title ?? "???" }}
          </a>
          <span class="ml-1.5 text-xs text-gray-400">
            {{ humanizeBytes(Number(optimisticValue?.size ?? 0)) }}
          </span>
          <i
            v-if="upload != null && upload.isActive.value"
            class="fas fa-spinner-third ml-1.5 animate-spin text-gray-400"
          />
        </span>
      </div>
      <!-- Not ready -->
      <div
        v-else
        class="flex h-full w-full flex-col items-center justify-center"
        :class="[download?.status.value == FileStatus.FAILED ? 'text-warning-600' : 'text-gray-400']"
      >
        <span>
          <IconInline
            v-bind="getFileStatusIcon(download?.status.value ?? FileStatus.PENDING)"
            :class="[
              download?.status.value == FileStatus.PREPARING || download?.status.value == FileStatus.TRANSFERRING
                ? 'animate-spin'
                : '',
            ]"
          />
          <template v-if="download?.filePtr">
            <span class="ml-1.5">{{ download.filePtr.title ?? "???" }}</span>
            <span class="ml-1.5 text-xs text-gray-400">
              {{ humanizeBytes(Number(download.filePtr.size ?? 0)) }}
            </span>
          </template>
          <span v-else class="ml-1.5">
            {{ getFileStatusName(download?.status.value ?? FileStatus.PENDING) }} {{ facetName }}
          </span>
        </span>
      </div>
      <!-- Overlay -->
      <div
        class="absolute top-0 h-full w-full"
        :class="upload?.isActive?.value ? 'bg-white bg-opacity-50 transition-colors duration-150' : ''"
      >
        <!-- Upload progress -->
        <div v-if="upload != null && upload.isActive.value" class="absolute top-0 w-full">
          <div
            class="h-1 transform rounded-full bg-primary-400 transition-transform duration-75"
            :style="{ width: upload.progress.value + '%' }"
          />
        </div>
        <!-- Controls -->
        <div
          class="absolute top-0 flex w-full flex-row justify-end gap-x-1 p-1.5 opacity-0 transition-colors duration-75 group-hover/inline:opacity-100"
        >
          <!-- Replace -->
          <button
            v-if="isInput && !isDisabled"
            class="rounded-2xl bg-white bg-opacity-50 px-1 text-gray-700 hover:bg-opacity-100 hover:text-primary-900"
            @click="fileInputRef?.click()"
          >
            <i class="fas fa-shuffle" />
          </button>
          <!-- Remove -->
          <button
            v-if="isInput && !isDisabled && !valueType?.isRequired"
            class="rounded-2xl bg-white bg-opacity-50 px-1 text-gray-700 hover:bg-opacity-100 hover:text-primary-900"
            @click="emit('update:modelValue', null)"
          >
            <i class="fas fa-xmark" />
          </button>
        </div>
      </div>
    </div>
  </ViewContentWrapper>
</template>
