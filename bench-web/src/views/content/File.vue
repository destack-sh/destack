<script lang="ts" setup>
import {
  FileStatus,
  getFileAcceptFromConstraint,
  getFileIconMaybe,
  uploadFile,
  useFileDownload,
  type FileUpload,
} from "@/language/file";
import { toCamelName } from "@/language/const";
import {
  BoxData,
  FileFormat,
  FileReferenceData,
  FileType,
  NodeType,
  ObjectType,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { bench, canvas, pkg, pkgConnection } from "@/system/space";
import { useDropZone } from "@/ui/drag";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE, IconInline } from "@/ui/icon";
import { type HoverMenuOptions, type PopoverContext } from "@/ui/popover";
import { toaster } from "@/ui/toast";
import { FILE_TYPE_BY_VIEW_TYPE } from "@/ui/view";
import { getElement } from "@/utils/element";
import { log } from "@/utils/log";
import { humanizeBytes } from "@/utils/string";
import { makeViewId, ViewContentWrapper, viewEmits, type ViewExposed, type ViewProps } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const FILE_POPOVER_WIDTH_MIN = 400;
const FILE_POPOVER_WIDTH_MAX = 800;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    modelValue?: FileReferenceData;
    size?: Partial<Pick<BoxData, "width" | "height">>;
  } & Partial<
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
  if (props.valueType?.constraint?.fileTypes?.length == 1) return props.valueType.constraint.fileTypes[0];
  else if (props.type != null) return FILE_TYPE_BY_VIEW_TYPE[props.type] ?? FileType.GENERIC;
  else return FileType.GENERIC;
});
const fileFormat = computed(() => {
  if (props.valueType?.constraint?.fileFormats?.length == 1) return props.valueType.constraint.fileFormats[0];
  else return undefined;
});
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
  } else if (fileType.value != FileType.GENERIC) {
    return toCamelName(FileType, fileType.value);
  } else {
    return "File";
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
      allowedFormats: fileFormat.value != null ? [fileFormat.value] : undefined,
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
defineExpose<ViewExposed>({
  self,
  id,
  interact: () => {
    if (props.modelValue != null) {
      openFile();
    } else {
      fileInputRef.value?.click();
    }
  },
});
</script>
<template>
  <ViewContentWrapper v-bind="props" :class="[size?.height != null ? 'h-full' : 's']">
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
      v-hovermenu="
        {
          isEnabled: () =>
            optimisticValue != null &&
            [FileType.TEXT, FileType.CODE, FileType.IMAGE, FileType.AUDIO].includes(optimisticValue.type),
          popover: (context: PopoverContext) => ({
            component: ViewType.FILE,
            props: {
              ...(props as ViewProps),
              title: undefined,
              size: {
                metatype: ObjectType.BOX,
                width: Math.max(
                  FILE_POPOVER_WIDTH_MIN,
                  Math.min(FILE_POPOVER_WIDTH_MAX, getElement(context.triggerElement)!.getBoundingClientRect().width),
                ),
              },
              isInline: true,
              isInput: false,
              modelValue: optimisticValue,
            },
          }),
        } as HoverMenuOptions
      "
      class="group/dropdown flex w-full flex-row items-center truncate rounded transition-all duration-75 data-[popover=true]:border-gray-300"
      :class="[
        variant != Variant.STEALTH ? 'border px-2.5 py-1' : '',
        variant == Variant.STEALTH && isInDropZone ? 'bg-primary-100' : '',
        isInDropZone
          ? 'border-primary-900 outline outline-1 outline-primary-900'
          : 'border-gray-200 hover:border-gray-300',
      ]"
      @click.stop="download?.getUrl.value != null ? openFile() : fileInputRef!.click()"
    >
      <!-- Current value -->
      <span v-if="optimisticValue != null" :class="loadFailed ? 'text-danger-600' : 'text-gray-700'">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" :class="loadFailed ? 'text-danger-600' : 'text-gray-700'" />
        <a
          class="max-w-full truncate decoration-gray-300 underline-offset-3 group-hover/dropdown:underline group-hover/dropdown:decoration-primary-900"
          :class="download?.getUrl.value != null ? 'hover:underline' : ''"
          :href="download?.getUrl.value ?? undefined"
          target="_blank"
        >
          {{ optimisticValue.title ?? "???" }}
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
        :class="isInDropZone ? 'text-primary-900' : 'text-gray-400'"
      >
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
      <!-- Controls -->
      <div v-if="!isDisabled && isInput" class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1 pl-1.5">
        <!-- Clear -->
        <button
          v-if="modelValue != null && !valueType?.isRequired"
          class="px-0.5 text-gray-400 opacity-0 hover:text-primary-900 group-hover/dropdown:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        >
          <i class="fas fa-xmark" />
        </button>
        <!-- Select -->
        <button class="px-0.5 text-gray-400 hover:text-primary-900" @click.stop="fileInputRef?.click()">
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
        variant != Variant.STEALTH ? 'border px-2.5 py-1' : '',
        variant == Variant.STEALTH && isInDropZone ? 'bg-primary-100' : '',
        isInDropZone
          ? 'border-primary-900 text-primary-900 outline outline-2 outline-primary-900'
          : 'border-gray-200 text-gray-400 hover:border-gray-300',
      ]"
      @click.stop="fileInputRef!.click()"
    >
      <span
        class="select-none transition-colors duration-75"
        :class="variant == Variant.STEALTH && !isInDropZone ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5" />
        <span>Upload {{ facetName }}</span>
      </span>
    </div>

    <!-- Inline value -->
    <div
      v-else
      ref="containerRef"
      class="group/inline relative flex h-full w-full flex-col justify-center rounded border-gray-200"
      :class="[
        variant != Variant.STEALTH ? 'border py-1' : '',
        variant == Variant.STEALTH && isInDropZone ? 'bg-primary-100' : '',
        isInDropZone ? 'border-primary-900 outline outline-2 outline-primary-900' : '',
      ]"
    >
      <!-- NOTE :Incomplete: proper file content views (image with proper size & thumbnail, audio, ...) -->
      <!-- Image File -->
      <div v-if="optimisticValue?.type == FileType.IMAGE && download?.getUrl.value != null">
        <img
          :key="download.getUrl.value"
          :src="download.getUrl.value"
          :alt="optimisticValue?.title ?? '???'"
          class="h-full w-full rounded object-contain object-center"
          :style="{
            maxWidth: size?.width != null ? `${size.width}px` : undefined,
            maxHeight: size?.height != null ? `${size.height - 8}px` : undefined,
            aspectRatio: (download?.file.value ?? modelValue)?.aspectRatio ?? undefined,
          }"
        />
      </div>
      <!-- Generic File -->
      <div
        v-else-if="optimisticValue != null"
        class="flex h-full w-full items-center justify-center text-center"
        :class="[loadFailed ? 'text-danger-600' : '']"
      >
        <span>
          <IconInline
            v-bind="getFileIconMaybe(optimisticValue) ?? facetIcon"
            :class="loadFailed ? 'text-danger-600' : 'text-gray-700'"
          />
          <a
            class="ml-1.5 decoration-gray-300 underline-offset-3 hover:decoration-primary-900"
            :class="download?.getUrl.value != null ? 'hover:underline' : ''"
            :href="download?.getUrl.value ?? undefined"
            target="_blank"
          >
            {{ optimisticValue?.title ?? "???" }}
          </a>
          <span v-if="optimisticValue.size != null" class="ml-1.5 text-xs text-gray-400">
            {{ humanizeBytes(Number(optimisticValue?.size)) }}
          </span>
          <!-- Transferring -->
          <i
            v-if="
              (upload != null && upload.isActive.value) ||
              (download != null && download.isActive.value && variant != Variant.STEALTH)
            "
            class="fas fa-spinner-third ml-2 animate-spin text-gray-400"
          />
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
          <template v-if="download?.filePtr">
            <span class="ml-1.5">{{ download.filePtr.title ?? "???" }}</span>
            <span v-if="download.filePtr.size != null" class="ml-1.5 text-xs text-gray-400">
              {{ humanizeBytes(Number(download.filePtr.size)) }}
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
          class="absolute right-0 top-0 m-1 flex flex-row justify-end gap-x-1 rounded border border-gray-200 bg-white px-1 py-0.5 opacity-0 transition-colors duration-75 group-hover/inline:text-gray-700 group-hover/inline:opacity-100"
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
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-primary-900"
            @click="openFile()"
          >
            <i class="fas fa-magnifying-glass-plus" />
          </button>
          <!-- Replace -->
          <button
            v-if="isInput && !isDisabled"
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-primary-900"
            @click="fileInputRef?.click()"
          >
            <i class="fas fa-shuffle" />
          </button>
          <!-- Remove -->
          <button
            v-if="isInput && !isDisabled && !valueType?.isRequired"
            class="rounded px-1 transition-colors duration-75 hover:bg-gray-100 hover:text-primary-900"
            @click="emit('update:modelValue', null)"
          >
            <i class="fas fa-xmark" />
          </button>
        </div>
      </div>
    </div>
  </ViewContentWrapper>
</template>
