<script lang="ts" setup>
import { RemoteObjectStatus, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { humanizeBytes, useObjects, type ObjectRecord } from "@/state/object";
import { TypeFlag } from "@/state/runtime";
import { useRelativeDropZone } from "@/utils/drop";
import { ArrowPathIcon, ArrowUpTrayIcon, DocumentArrowUpIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{
  type: SimpleType;
  modelValue: ObjectRecord[];
  readonly?: boolean;
  active?: boolean;
  preview?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: ObjectRecord[]): void;
  (e: "dropFiles", p: "above" | "below", v: File[]): void;
}>();

const isArray = computed(() => props.type.flags & TypeFlag.IsArray);
const objects = useObjects();
const editor = useEditorState();
const ongoingUploads = ref(0);

const fileChooserRef = ref<HTMLInputElement | null>(null);
const dropZoneRef = ref<HTMLDivElement>();
const { isOverDropZone: dragOver } = useRelativeDropZone(
  dropZoneRef,
  ["File", "Record"],
  onDrop,
  computed(() => !props.readonly)
);

function onDrop(files: File[] | { type: string; id: string } | null) {
  if (!Array.isArray(files)) {
    return; // ignore
  }
  // upload into here
  if (isArray.value) {
    files.forEach(doUpload);
  } else if (files.length > 0) {
    doUpload(files[0]);
    if (files.length > 1) {
      emit("dropFiles", "below", files.slice(1));
    }
  }
}

async function doUpload(file: File | null) {
  if (file == null || editor.currentProjectId == null) return;
  ongoingUploads.value++;
  if (!isArray.value) {
    await emit("update:modelValue", []);
  }
  function onUpdate(val: ObjectRecord | null) {
    // if single, replace value
    if (!isArray.value) {
      emit("update:modelValue", val == null ? [] : [val]);
    } else if (val != null) {
      // replace specific value or append
      const idx = props.modelValue.findIndex((v) => v.id == val?.id);
      if (idx >= 0) {
        emit("update:modelValue", [...props.modelValue.slice(0, idx), val, ...props.modelValue.slice(idx + 1)]);
      } else {
        emit("update:modelValue", [...props.modelValue, val]);
      }
    }
  }
  await objects.upload(editor.currentProjectId, file, onUpdate);
  ongoingUploads.value--;
}

async function open(file: ObjectRecord) {
  if (file.status != RemoteObjectStatus.Available) return;
  // open file (in new tab)
  const presignedGet = await objects.getPresignedGet(file.id);
  window.open(presignedGet, "_blank");
}

function focus() {
  if (props.modelValue.length == 0) {
    fileChooserRef.value?.click();
  } else {
    // ???
  }
}

function blur() {
  // ???
}

defineExpose({
  focus,
  blur,
  click: () => props.readonly || fileChooserRef.value?.click(),
});
</script>
<template>
  <!-- Entire thing is drop zone -->
  <div
    ref="dropzoneRef"
    class="flex w-full flex-row flex-wrap gap-1"
    :class="{
      'rounded-sm border border-dashed border-orange-500': dragOver,
      'border border-transparent': !dragOver,
      'justify-center': modelValue.length == 0,
    }"
  >
    <!-- Existing files -->
    <span
      v-for="file in modelValue"
      :key="file.id"
      class="group flex flex-row items-center gap-1.5 hover:cursor-pointer"
      @click="open(file)"
    >
      <!-- :FileStyle -->
      <component
        :is="file.status == RemoteObjectStatus.Uploading ? ArrowPathIcon : DocumentArrowUpIcon"
        class="h-4 w-4 text-gray-700"
        :class="file.status == RemoteObjectStatus.Uploading ? 'animate-spin' : ''"
      />
      <span class="text-gray-700 underline-offset-4 group-hover:underline">{{ file.name }}</span>
      <span class="text-xs text-gray-400">{{ humanizeBytes(file?.contentLength) }}</span>
    </span>
    <!-- Ensure there's always some content -->
    <template v-if="modelValue.length == 0">&nbsp;</template>
    <!-- Upload button -->
    <button
      v-if="!readonly && active && (isArray || modelValue.length == 0)"
      :disabled="ongoingUploads > 0"
      class="self-end justify-self-end rounded-sm border-gray-300 px-0.5 hover:bg-orange-100"
      :class="ongoingUploads ? 'animate-spin' : ''"
      @click.stop.prevent="fileChooserRef?.click()"
    >
      <component :is="ongoingUploads ? ArrowPathIcon : ArrowUpTrayIcon" class="h-4 w-4 text-gray-400" />
    </button>
    <!-- Actual file chooser (unstylable, hidden) -->
    <input
      :disabled="props.readonly"
      ref="fileChooserRef"
      id="fileChooser"
      type="file"
      class="hidden"
      @change="(e) => doUpload(e.target?.files?.[0])"
    />
  </div>
</template>
