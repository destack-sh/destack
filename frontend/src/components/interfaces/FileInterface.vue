<script lang="ts" setup>
import { useElementRefs } from "@/components/cells/grid";
import { RemoteObjectStatus, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { humanizeBytes, useObjects, type ObjectRecord } from "@/state/object";
import { TypeFlag } from "@/state/runtime";
import { useRelativeDropZone } from "@/utils/drop";
import { ArrowPathIcon, ArrowUpTrayIcon, DocumentArrowUpIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";

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

const fileRefs = useElementRefs();
const fileChooserRef = ref<HTMLInputElement | null>(null);
const uploadButtonRef = ref<HTMLButtonElement | null>(null);
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
      // refocus since button may be gone
      nextTick(() => (val == null ? uploadButtonRef.value?.focus() : fileRefs.focus(val.id)));
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

function remove(file: ObjectRecord) {
  const fileIndex = props.modelValue.findIndex((f) => f.id == file.id);
  emit(
    "update:modelValue",
    props.modelValue.filter((f) => f.id != file.id)
  );
  if (!isArray.value || props.modelValue.length == 1) {
    nextTick(() => uploadButtonRef.value?.focus());
  } else {
    fileRefs.focus(props.modelValue[fileIndex - 1]?.id);
  }
}

function focus() {
  if (props.modelValue.length == 0) {
    fileChooserRef.value?.click();
  } else if (isArray.value) {
    uploadButtonRef.value?.focus();
  } else {
    fileRefs.focus(props.modelValue.slice(-1)[0]?.id);
  }
}

function blur() {
  // nothing to do?
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
    ref="dropZoneRef"
    class="group/iface flex w-full flex-row flex-wrap gap-x-2.5 gap-y-0.5"
    :class="{
      'rounded-sm border border-dashed border-orange-500': dragOver,
      'border border-transparent': !dragOver,
      'justify-center': modelValue.length == 0,
      'min-w-[300px]': !preview,
    }"
  >
    <!-- Existing files -->
    <!-- :FileStyle -->
    <div
      :ref="(el: any) => fileRefs.registerRef(file.id, el)"
      v-for="(file, i) in modelValue"
      tabindex="-1"
      :key="file.id"
      class="group/file flex flex-row items-center gap-1.5 rounded-sm hover:cursor-pointer focus:bg-orange-100 focus:outline-none"
      @click.stop="open(file)"
      @keydown.enter.stop.prevent="open(file)"
      @keydown.right.stop.prevent="
        i == modelValue.length - 1 ? uploadButtonRef?.focus() : fileRefs.focus(modelValue[i + 1]?.id)
      "
      @keydown.left.stop.prevent="i == 0 ? null : fileRefs.focus(modelValue[i - 1]?.id)"
      @keydown.delete.stop.prevent="remove(file)"
    >
      <!-- File status & info -->
      <component
        :is="file.status == RemoteObjectStatus.Uploading ? ArrowPathIcon : DocumentArrowUpIcon"
        class="h-4 w-4 text-gray-700"
        :class="file.status == RemoteObjectStatus.Uploading ? 'animate-spin' : ''"
      />
      <span class="flex flex-row items-baseline gap-1.5">
        <span class="text-gray-900 underline-offset-4 group-hover/file:underline">{{ file.name }}</span>
        <span class="text-xs text-gray-400">{{ humanizeBytes(file?.content_length) }}</span>
      </span>
      <!-- Delete button -->
      <button
        v-if="!preview"
        class="text-gray-300 focus:text-gray-700 group-hover/file:text-gray-500"
        @click.stop.prevent="remove(file)"
      >
        x
      </button>
    </div>
    <!-- Ensure there's always some content -->
    <template v-if="modelValue.length == 0">&nbsp;</template>
    <!-- Upload button -->
    <button
      ref="uploadButtonRef"
      v-if="!readonly && (isArray || modelValue.length == 0)"
      :disabled="ongoingUploads > 0"
      class="self-end justify-self-end rounded-sm border-gray-300 px-0.5 transition hover:bg-orange-100 focus:bg-orange-100 focus:outline-none group-focus-within/iface:opacity-100 group-hover/iface:opacity-100"
      :class="[ongoingUploads ? 'animate-spin' : '', preview ? 'opacity-0' : '']"
      @click.stop.prevent="fileChooserRef?.click()"
      @keydown.enter.stop.prevent="fileChooserRef?.click()"
      @keydown.left.stop.prevent="fileRefs.focus(modelValue.slice(-1)[0]?.id)"
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
      @click.stop
    />
  </div>
</template>
