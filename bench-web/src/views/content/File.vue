<script lang="ts" setup>
import { ViewData, NodeType, FileReferenceData, ViewType, FileType, FileFormat } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef } from "vue";
import { toCamelName } from "@/system/lang";
import { ICON_BY_FILE_FORMAT, ICON_BY_FILE_TYPE, IconInline } from "@/system/icon";
import { humanizeBytes } from "@/utils/string";
import { FILE_TYPE_BY_VIEW_TYPE } from "@/system/view";
import { useDropZone } from "@/utils/drag";
import { extractFileInfo } from "@/system/file";

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

async function onFileSelected(event: DragEvent) {
  // nocheckin: use file
  if (event.dataTransfer?.files.length == 0) return;
  const file = event.dataTransfer!.files[0];
  const fileData = await extractFileInfo(file);
}

const { isInDropZone } = useDropZone({
  name: "file",
  container: containerRef,
  kinds: ["file"],
  onDrop: (dragged, event) => {
    if (dragged.kind == "file") {
      onFileSelected(event);
    }
  },
  isEnabled: computed(() => props.isInput && !props.isDisabled),
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- nocheckin: File -->
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
      <input ref="fileInputRef" type="file" class="hidden" @change="onFileSelected" />
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
      <input ref="fileInputRef" type="file" class="hidden" @change="onFileSelected" />
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
