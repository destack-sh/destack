<script lang="ts" setup>
import { RemoteObjectStatus, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { humanizeBytes, useObjects, type ObjectRecord } from "@/state/object";
import { useRelativeDropZone } from "@/utils/drop";
import { DocumentArrowUpIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: ObjectRecord | null;
  type: SimpleType;
  readonly: boolean;
  active?: boolean;
  supportsDrop?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: ObjectRecord | null): void;
  (e: "dropFiles", p: "above" | "below", v: File[]): void;
}>();

const amUploading = ref(false);
const isFileUploaded = computed(() => props.modelValue?.status == RemoteObjectStatus.Available);
const isFileUploading = computed(
  () =>
    props.modelValue?.status == RemoteObjectStatus.Uploading ||
    props.modelValue?.status == RemoteObjectStatus.Prepared ||
    amUploading.value
);

const objects = useObjects();
const editor = useEditorState();
const fileChooserRef = ref<HTMLInputElement | null>(null);
const dropZoneRef = ref<HTMLDivElement>();
const {
  isOverDropZone: dragOver,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(dropZoneRef, ["File", "Record"], onDrop);

function onDrop(files: File[] | null) {
  if (isFileUploaded.value && props.supportsDrop) {
    // drop above or below
    if (files != null && files.length > 0) {
      emit("dropFiles", dragInTopHalf.value ? "above" : "below", files);
    }
  } else if (files != null && files.length > 0) {
    // upload into here
    beginUpload(files[0]);
    if (files.length > 1 && props.supportsDrop) {
      emit("dropFiles", "below", files);
    }
  }
}

function clear() {
  emit("update:modelValue", null);
}

async function open() {
  if (isFileUploaded.value) {
    // open file (in new tab)
    const presignedGet = await objects.getPresignedGet(props.modelValue?.id);
    window.open(presignedGet, "_blank");
  } else {
    // open file chooser
    fileChooserRef.value?.click();
  }
}

async function beginUpload(file: File | null) {
  if (file == null) {
    return; // ignore
  }
  if (editor.currentProjectId == null) {
    throw new Error("no active project");
  }
  if (isFileUploaded.value) {
    throw new Error("file already uploaded");
  }
  amUploading.value = true;
  emit("update:modelValue", null);
  await objects.upload(editor.currentProjectId, file, (val) => emit("update:modelValue", val));
  amUploading.value = false;
}

defineExpose({
  open,
});
</script>
<template>
  <!-- File drop/select zone -->
  <label
    v-if="!isFileUploaded && !isFileUploading"
    ref="dropZoneRef"
    for="fileChooser"
    class="group inline-block w-full cursor-pointer"
    :class="{
      'rounded-sm border border-dashed border-orange-500': dragOver,
      'border border-transparent': !dragOver,
    }"
  >
    <span :class="active || dragOver ? '' : 'invisible group-hover:visible'">file</span>
    <span class="ml-1" v-if="dragOver">(drop to upload)</span>
    <input
      ref="fileChooserRef"
      id="fileChooser"
      type="file"
      class="hidden"
      @change="(e) => beginUpload(e.target?.files?.[0])"
    />
  </label>
  <!-- Currently uploading -->
  <span v-else-if="isFileUploading" class="group flex flex-row items-center text-gray-600">
    uploading...
    <!-- Clear button (if someone else is uploading, may be stuck) -->
    <button
      v-if="!amUploading && !readonly"
      class="ml-2 rounded-sm px-0.5 text-xs hover:bg-gray-200 group-hover:text-gray-700"
      @click.prevent.stop="clear"
      :class="active ? 'text-gray-300' : 'text-transparent'"
    >
      x
    </button>
  </span>
  <!-- Existing file -->
  <!-- TODO @Feature @UX: make file view openable and prettier -->
  <!-- TODO @Feature @UX: preview files, images, etc.! -->
  <span
    v-else
    ref="dropZoneRef"
    class="text- black group relative inline-block h-full w-full hover:cursor-pointer"
    :class="{ 'bg-orange-100': dragOver && supportsDrop }"
    @click="open"
  >
    <!-- Statement drag & drop indicator (top/bottom) -->
    <div
      v-if="!readonly && dragOver && dragInTopHalf"
      class="duration-50 absolute -top-1 left-0 h-1 w-full bg-orange-300 transition-colors"
    />
    <div
      v-if="!readonly && dragOver && dragInBottomHalf"
      class="duration-50 absolute -bottom-1 left-0 h-1 w-full bg-orange-300 transition-colors"
    />
    <!-- File ifo -->
    <DocumentArrowUpIcon class="inline-block h-4 w-4" />
    <span v-if="modelValue" class="ml-1 underline-offset-4 group-hover:underline">{{ modelValue.name }}</span>
    <span class="ml-2 text-xs text-gray-400" v-if="modelValue">{{ humanizeBytes(modelValue?.contentLength) }}</span>
    <!-- Clear button -->
    <button
      v-if="!readonly"
      class="ml-2 rounded-sm px-0.5 text-xs hover:bg-gray-200 group-hover:text-gray-700"
      @click.prevent.stop="clear"
      :class="active ? 'text-gray-300' : 'text-transparent'"
    >
      x
    </button>
  </span>
</template>
