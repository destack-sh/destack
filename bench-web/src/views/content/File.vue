<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import {
  FileStatus,
  getFileAcceptFromConstraint,
  getFileIconMaybe,
  INLINE_FILE_TYPES,
  uploadFile,
  useFileDownload,
  type FileUpload,
} from "@/language/resource/file";
import {
  FileFormat,
  FileType,
  FileTypeOptionInfo,
  NodeReferenceData,
  NodeType,
  ObjectType,
  RectangleData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { bench, canvas, pkgConnection } from "@/system/space";
import { useDropZone } from "@/ui/drag";
import { IconInline, makeIcon } from "@/ui/icon";
import { type HoverMenuOptions, type PopoverContext } from "@/ui/popover";
import { toaster } from "@/ui/toast";
import { FILE_TYPE_BY_VIEW_TYPE } from "@/ui/view";
import { getElement } from "@/utils/element";
import { log } from "@/utils/log";
import { humanizeBytes } from "@/utils/string";
import { type ViewEmits, type ViewExpose, type ViewProps } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const FILE_POPOVER_WIDTH_MIN = 400;
const FILE_POPOVER_WIDTH_MAX = 800;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: NodeReferenceData;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
  } & Partial<
    Pick<
      ViewData,
      "type" | "name" | "title" | "icon" | "valueType" | "nodePtr" | "isInput" | "isInline" | "isDisabled" | "isMinimal"
    >
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const fileType = computed(() => {
  if (props.valueType?.constraint?.nodeSubtypes?.length == 1)
    return props.valueType.constraint.nodeSubtypes[0] as FileType;
  else if (props.type != null) return FILE_TYPE_BY_VIEW_TYPE[props.type] ?? FileType.GENERIC;
  else return FileType.GENERIC;
});
const facetIcon = computed(() => {
  return makeIcon(FileTypeOptionInfo[fileType.value]!.icon!);
});
const facetName = computed(() => {
  if (fileType.value != FileType.GENERIC) {
    return toCamelName(FileType, fileType.value);
  } else {
    return "File";
  }
});

//
// Interaction
//

const HOVER_MENU: HoverMenuOptions = {
  reference: "trigger",
  isEnabled: () =>
    optimisticValue.value != null &&
    [FileType.TEXT, FileType.CODE, FileType.IMAGE, FileType.AUDIO].includes(optimisticValue.value.type),
  popover: (context: PopoverContext) => ({
    kind: "view",
    component: ViewType.FILE,
    props: {
      ...(props as ViewProps),
      title: undefined,
      size: {
        metatype: ObjectType.RECTANGLE,
        width: Math.max(
          FILE_POPOVER_WIDTH_MIN,
          Math.min(FILE_POPOVER_WIDTH_MAX, getElement(context.element)!.getBoundingClientRect().width),
        ),
      },
      isInline: true,
      isInput: false,
      modelValue: optimisticValue.value,
    },
  }),
};

const fileInputRef = ref<HTMLInputElement | null>(null);
const containerRef = ref<HTMLElement | null>(null);
const upload: Ref<FileUpload | null> = ref(null);
const download = useFileDownload(toRef(props, "modelValue"));
const optimisticValue = computed(() => upload.value?.file.value ?? download.value?.file.value);
const loadFailed = computed(() => download.value?.status.value == FileStatus.FAILED);

async function onFileSelected(files: File[]) {
  if (files.length == 0) return;
  if (bench.value == null) throw new Error("no current bench");
  const content = files[0];
  // NOTE :Incomplete: uploaded file should be attributed to closest ancestor block, not package (?)
  try {
    upload.value = uploadFile(() => pkgConnection.tx, content, {
      bench: bench.value,
      allowedTypes: fileType.value != FileType.GENERIC ? [fileType.value] : undefined,
    });
    await upload.value.completion.wait();
    if (upload.value.file.value == null) throw new Error("missing file in upload");
    const fileRef = toNodeRef(upload.value.file.value);
    emit("update:modelValue", fileRef);
  } catch (e) {
    log.error("file.upload.error", upload, e);
    toaster.error({ title: "Upload Failed", text: `'${content.name}': ${(e as any)?.message ?? "unknown error"}` });
  } finally {
    upload.value = null;
  }
}

function openFile() {
  if (download.value?.getUrl.value == null) return;
  window.open(download.value.getUrl.value, "_blank");
}

// NOTE :UX: should constrain drop mime types to file types
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
defineExpose<ViewExpose>({
  self,
  id,
  interact: () => {
    if (props.modelValue != null) {
      return false; // used to openFile here, but that's a bit annoying
    } else {
      fileInputRef.value?.click();
    }
  },
});
</script>
<template>
  <div v-bind="props" :class="[size?.height != null ? 'h-full' : '']">
    <!-- Actual file input (hidden) -->
    <input
      ref="fileInputRef"
      type="file"
      class="hidden"
      :accept="getFileAcceptFromConstraint(fileType, valueType?.constraint)"
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
      v-hovermenu="HOVER_MENU"
      class="group flex w-full flex-row items-center truncate rounded transition-all duration-75 data-[popover=true]:border-gray-200"
      :class="[
        !isMinimal ? 'border px-2.5 py-1' : '',
        isMinimal && isInDropZone ? 'bg-gray-100' : '',
        isInDropZone ? 'border-gray-400 outline outline-1 outline-gray-400' : 'border-gray-200 hover:border-gray-200',
        isMinimal && optimisticValue == null && !isInDropZone ? 'opacity-0 hover:opacity-100' : '',
      ]"
      @click.stop="download?.getUrl.value != null ? openFile() : fileInputRef!.click()"
    >
      <!-- Current value -->
      <span v-if="optimisticValue != null" :class="[loadFailed ? 'text-danger-600' : 'text-gray-700']">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" :class="loadFailed ? 'text-danger-600' : 'text-gray-700'" />
        <a
          class="max-w-full truncate decoration-gray-300 underline-offset-3 group-hover:underline group-hover:decoration-gray-700"
          :class="download?.getUrl.value != null ? 'hover:underline' : ''"
          :href="download?.getUrl.value ?? undefined"
          target="_blank"
        >
          {{ optimisticValue.name ?? "???" }}
        </a>
        <span class="ml-1.5 flex-shrink-0 text-xs text-gray-400">
          {{ humanizeBytes(Number(optimisticValue.size)) }}
        </span>
        <!-- Uploading -->
        <i
          v-if="upload != null && upload.isActive.value"
          class="fas fa-spinner-third ml-1.5 animate-spin text-gray-400"
        />
      </span>
      <span
        v-else
        class="select-none transition-colors duration-75"
        :class="[isInDropZone ? 'text-gray-700' : 'text-gray-400 group-hover:text-gray-700']"
      >
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
      <!-- Controls -->
      <div v-if="!isDisabled && isInput" class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1 pl-1.5">
        <!-- Clear -->
        <button
          v-if="modelValue != null && !valueType?.isRequired"
          class="px-0.5 text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        >
          <i class="fas fa-xmark" />
        </button>
        <!-- Select -->
        <button class="px-0.5 text-gray-400 hover:text-gray-700" @click.stop="fileInputRef?.click()">
          <i class="fas fa-caret-down" />
        </button>
      </div>
    </button>

    <!-- Inline drop area -->
    <div
      v-else-if="optimisticValue == null"
      ref="containerRef"
      class="group flex h-full w-full cursor-pointer flex-col justify-center rounded text-center transition-all duration-75"
      :class="[
        !isMinimal ? 'border px-2.5 py-1' : '',
        isMinimal && isInDropZone ? 'bg-gray-100' : '',
        isInDropZone
          ? 'border-gray-400 text-gray-700 outline outline-2 outline-gray-400'
          : 'border-gray-200 text-gray-400 hover:border-gray-200',
      ]"
      @click.stop="fileInputRef!.click()"
    >
      <span
        class="select-none transition-colors duration-75"
        :class="isMinimal && !isInDropZone ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
    </div>

    <!-- Inline value -->
    <div
      v-else
      ref="containerRef"
      class="group relative flex h-full w-full flex-col justify-center rounded border-gray-200"
      :class="[
        !isMinimal ? 'border' : '',
        !isMinimal && !optimisticValue ? 'py-1' : '',
        isMinimal && isInDropZone ? 'bg-gray-100' : '',
        isInDropZone ? 'border-gray-700 outline outline-2 outline-gray-700' : '',
      ]"
      data-suppress-drag="select"
    >
      <!-- NOTE :Incomplete: proper file content views (image with proper size & thumbnail, audio, ...) -->
      <!-- Image File -->
      <div v-if="optimisticValue?.type == FileType.IMAGE && download?.getUrl.value != null">
        <img
          :key="download.getUrl.value"
          :src="download.getUrl.value"
          :alt="optimisticValue?.name ?? '???'"
          class="h-full w-full rounded object-contain object-center"
          :style="{
            maxWidth: size?.width != null ? `${size.width}px` : undefined,
            maxHeight: size?.height != null ? `${size.height - 8}px` : undefined,
            aspectRatio: optimisticValue?.aspectRatio ?? undefined,
          }"
        />
      </div>
      <!-- Generic File -->
      <div
        v-else-if="optimisticValue != null"
        class="flex h-full w-full items-center justify-center text-center"
        :class="[
          loadFailed ? 'text-danger-600' : '',
          (upload != null && upload.isActive.value) || (download != null && download.isActive.value && !isMinimal)
            ? 'animate-pulse'
            : '',
        ]"
        :style="{
          maxWidth: size?.width != null ? `${size.width}px` : undefined,
          maxHeight: size?.height != null ? `${size.height - 8}px` : undefined,
          aspectRatio: INLINE_FILE_TYPES.includes(optimisticValue.type)
            ? (download?.file.value?.aspectRatio ?? undefined)
            : undefined,
        }"
      >
        <span>
          <IconInline
            v-bind="getFileIconMaybe(optimisticValue) ?? facetIcon"
            :class="loadFailed ? 'text-danger-600' : 'text-gray-700'"
          />
          <a
            class="ml-1.5 decoration-gray-300 underline-offset-3 hover:decoration-gray-700"
            :class="download?.getUrl.value != null ? 'hover:underline' : ''"
            :href="download?.getUrl.value ?? undefined"
            target="_blank"
          >
            {{ optimisticValue?.name ?? "???" }}
          </a>
          <span v-if="optimisticValue.size != null" class="ml-1.5 text-xs text-gray-400">
            {{ humanizeBytes(Number(optimisticValue?.size)) }}
          </span>
        </span>
      </div>
      <!-- Not ready -->
      <div
        v-else
        class="flex h-full w-full flex-row items-center justify-center"
        :class="[loadFailed ? 'text-warning-600' : 'text-gray-400']"
      >
        <span>
          <!-- Uploading -->
          <i v-if="upload != null && upload.isActive.value" class="fas fa-spinner-third animate-spin text-gray-400" />
          <i
            v-else-if="download?.status.value == FileStatus.FAILED"
            class="fas fa-circle-exclamation text-danger-600"
          />
          <template v-if="download?.file.value">
            <span class="ml-1.5">{{ download.file.value.name ?? "???" }}</span>
            <span v-if="download.file.value.size != null" class="ml-1.5 text-xs text-gray-400">
              {{ humanizeBytes(Number(download.file.value.size)) }}
            </span>
          </template>
          <span v-else class="ml-1.5">{{ facetName }}</span>
        </span>
      </div>

      <!-- Overlay -->
      <div
        class="absolute top-0 w-full"
        :class="upload?.isActive?.value ? 'h-full bg-white bg-opacity-50 transition-colors duration-150' : ''"
      >
        <!-- Upload progress -->
        <div v-if="upload != null && upload.isActive.value" class="absolute top-0 w-full">
          <div
            class="h-1 transform rounded-full bg-gray-400 transition-transform duration-75"
            :style="{ width: upload.progress.value + '%' }"
          />
        </div>
        <!-- Meta/Controls -->
        <div
          v-if="!isMinimal"
          class="absolute right-0 top-0 m-1 flex flex-row justify-end gap-x-1 rounded border border-gray-200 bg-white px-1 py-0.5 opacity-0 transition-colors duration-75 group-hover:text-gray-700 group-hover:opacity-100"
        >
          <!-- Format -->
          <span v-if="optimisticValue?.format" class="">
            {{ FileFormat[optimisticValue.format].toUpperCase().replace(/_/g, " ") }}
          </span>
          <!-- Size -->
          <span v-if="optimisticValue != null && optimisticValue?.type == FileType.IMAGE" class="text-gray-400">
            ({{ humanizeBytes(Number(optimisticValue.size)) }})
          </span>
          <!-- Focus -->
          <button
            v-if="isInput && !isDisabled"
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
            @click="openFile()"
          >
            <i class="fas fa-magnifying-glass-plus" />
          </button>
          <!-- Replace -->
          <button
            v-if="isInput && !isDisabled"
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
            @click="fileInputRef?.click()"
          >
            <i class="fas fa-shuffle" />
          </button>
          <!-- Remove -->
          <button
            v-if="isInput && !isDisabled && !valueType?.isRequired"
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
            @click="emit('update:modelValue', null)"
          >
            <i class="fas fa-xmark" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
