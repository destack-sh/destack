<script lang="ts" setup>
import { RemoteObjectStatus, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { humanizeBytes, useObjects, type ObjectRecord } from "@/state/object";
import { useOperations } from "@/state/operations";
import { DocumentArrowUpIcon } from "@heroicons/vue/24/outline";
import { useDropZone } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: ObjectRecord | null;
  type: SimpleType;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: ObjectRecord | null): void;
}>();

const preparingUpload = ref(false);
const isFileUploaded = computed(() => props.modelValue?.status == RemoteObjectStatus.Available);
const isFileUploading = computed(
  () => props.modelValue?.status == RemoteObjectStatus.Uploading || preparingUpload.value
);

const ops = useOperations();
const objects = useObjects();
const editor = useEditorState();
const fileChooserRef = ref<HTMLInputElement | null>(null);
const dropZoneRef = ref<HTMLDivElement>();
const { isOverDropZone } = useDropZone(dropZoneRef, beginUpload);

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

async function beginUpload(files: File[] | null) {
  if (files == null || files.length == 0) {
    return;
  }
  const file = files[0];
  if (editor.currentProjectId == null) {
    throw new Error("no active project");
  }
  if (isFileUploaded.value) {
    throw new Error("file already uploaded");
  }
  preparingUpload.value = true;
  await objects.upload(editor.currentProjectId, file, (val) => emit("update:modelValue", val));
  preparingUpload.value = false;
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
    class="inline-block w-full cursor-pointer"
    :class="{
      'rounded-sm border border-dashed border-orange-500': isOverDropZone,
      'border border-transparent': !isOverDropZone,
    }"
  >
    file
    <template v-if="isOverDropZone">(drop to upload)</template>
    <input
      ref="fileChooserRef"
      id="fileChooser"
      type="file"
      class="hidden"
      @change="(e) => beginUpload(e.target?.files)"
    />
  </label>
  <span v-else-if="isFileUploading" class="text-gray-600">uploading...</span>
  <!-- Existing file -->
  <!-- TODO @Feature @UX: make file view openable and prettier -->
  <span v-else class="group text-black hover:cursor-pointer" @click="open">
    <DocumentArrowUpIcon class="inline-block h-4 w-4" />
    <span v-if="modelValue" class="ml-1 underline-offset-4 group-hover:underline">{{ modelValue.name }}</span>
    <span class="ml-2 text-xs text-gray-400" v-if="modelValue">{{ humanizeBytes(modelValue?.contentLength) }}</span>
  </span>
</template>
