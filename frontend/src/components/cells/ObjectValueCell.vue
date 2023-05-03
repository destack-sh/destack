<script lang="ts" setup>
import { RemoteObjectStatus, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import type { BasicObject } from "@/state/object";
import { useOperations } from "@/state/operations";
import { useDropZone } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: BasicObject | null;
  type: SimpleType;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", v: BasicObject | null): void;
}>();

const isFileUploaded = computed(() => props.modelValue?.status == RemoteObjectStatus.Available);
const isFileUploading = computed(() => props.modelValue?.status == RemoteObjectStatus.Uploading);

const ops = useOperations();
const editor = useEditorState();
const fileChooserRef = ref<HTMLInputElement | null>(null);
const dropZoneRef = ref<HTMLDivElement>();
const { isOverDropZone } = useDropZone(dropZoneRef, beginUpload);

function open() {
  console.log("open", fileChooserRef.value); // TODO @Broken fix double (triple?) open on manual click
  fileChooserRef.value?.click();
}

function beginUpload(files: File[] | null) {
  if (files == null || files.length == 0) {
    return;
  }
  const file = files[0];
  if (editor.currentProjectId == null) {
    throw new Error("no active project");
  }
  upload(editor.currentProjectId, file);
}

async function upload(projectId: string, file: File) {
  // TODO @Cleanup: simplify and move to common object ops for re-use in other drop zones
  if (isFileUploaded.value) {
    throw new Error("file is already uploaded");
  }
  emit("update:modelValue", null);
  const ret = await ops.object.requestUpload(projectId, file);
  if (ret?.data?.requestUploadObject.__typename != "RemoteObject") {
    return; // ops errors are auto-handled
  }
  const remoteObject = ret.data.requestUploadObject;
  if (remoteObject.presignedPost == null) {
    throw new Error("no presigned post on remote object");
  }
  await ops.object.doUpload(remoteObject.id, remoteObject.presignedPost, file);
  // emit uploading state
  emit("update:modelValue", {
    id: remoteObject.id,
    name: remoteObject.name,
    contentLength: remoteObject.contentLength,
    contentType: remoteObject.contentType,
    sha512: remoteObject.sha512,
    status: RemoteObjectStatus.Uploading,
  });
  await ops.object.notifyUploaded(remoteObject.id);
  // emit uploaded state
  emit("update:modelValue", {
    id: remoteObject.id,
    name: remoteObject.name,
    contentLength: remoteObject.contentLength,
    contentType: remoteObject.contentType,
    sha512: remoteObject.sha512,
    status: RemoteObjectStatus.Available,
  });
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
  <span v-else-if="isFileUploading">uploading...</span>
  <!-- Existing file -->
  <span v-else>{{ modelValue?.name }}</span>
</template>
