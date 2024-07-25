<script lang="ts" setup>
import { FileFormat, FileReferenceData, FileType, NodeType, ViewData } from "@/proto/wire";
import { describeNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { uploadFiles, type FileUpload } from "@/system/file";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE, IconInline } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { canvas, pkg } from "@/system/space";
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

async function onFileSelected(files: File[]) {
  if (files.length == 0) return;
  if (pkg.value == null) throw new Error("no current package");
  const content = files[0];
  // NOTE :Incomplete: uploaded file should be attributed to closest ancestor block, not package (?)
  upload.value = uploadFiles([content], pkg.value)[0];
  await upload.value.completion.wait();
  if (upload.value.file.value == null) throw new Error("no file");
  emit("update:modelValue", toNodeReference(upload.value.file.value));
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
      <template v-if="modelValue != null">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5 text-gray-700" />
        <span>{{ modelValue.title ?? "???" }}</span>
        <span class="text-xs text-gray-400">{{ humanizeBytes(17000) }}</span>
      </template>
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
      v-else-if="modelValue == null"
      ref="containerRef"
      class="flex h-full min-h-[60px] w-full cursor-pointer flex-col justify-center rounded border px-2.5 py-1 text-center transition-all duration-75"
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
      class="h-full w-full rounded border border-gray-200"
      :class="[isInDropZone ? 'border-primary-400' : '']"
    >
      <!-- nocheckin: file view (image, audio, ... generic) -->
      {{ describeNode(modelValue) }}
    </div>
  </ViewContentWrapper>
</template>
