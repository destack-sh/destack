<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { BlobStatus, type Field } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { humanizeBytes, useObjects, type BlobRecord } from "@/state/blob";
import { TypeFlag } from "@/state/module";
import { useRelativeDropZone } from "@/utils/drop";
import { ArrowUpTrayIcon, DocumentArrowUpIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useNotifications } from "@/state/notifications";

const props = defineProps<{
  type: Field;
  modelValue: BlobRecord[];
  readonly?: boolean;
  active?: boolean;
  preview?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: BlobRecord[]): void;
  (e: "dropFiles", p: "above" | "below", v: File[]): void;
}>();

const isArray = computed(() => props.type.flags & TypeFlag.IS_ARRAY);
const objects = useObjects();
const bench = useBenchState();
const ongoingUploads = ref(0);
const notifications = useNotifications();

const blobRefs = useElementRefs();
const fileChooserRef = ref<HTMLInputElement | null>(null);
const uploadButtonRef = ref<HTMLButtonElement | null>(null);
const dropZoneRef = ref<HTMLDivElement>();
const { isOverDropZone: dragOver } = useRelativeDropZone(
  dropZoneRef,
  ["BrowserFile", "Record"],
  onDrop,
  computed(() => !props.readonly)
);

function onDrop(blobs: File[] | { type: string; id: string } | null) {
  if (!Array.isArray(blobs)) {
    return; // ignore
  }
  // upload into here
  if (isArray.value) {
    blobs.forEach(doUpload);
  } else if (blobs.length > 0) {
    doUpload(blobs[0]);
    if (blobs.length > 1) {
      emit("dropFiles", "below", blobs.slice(1));
    }
  }
}

async function doUpload(blob: File | null) {
  if (blob == null || bench.projectId == null) return;
  ongoingUploads.value++;
  function onUpdate(val: BlobRecord | null) {
    if (val == null) return;
    // if single, replace value
    if (!isArray.value) {
      // refocus since button may be gone
      nextTick(() => (val == null ? uploadButtonRef.value?.focus() : blobRefs.focus(val.id)));
      emit("update:modelValue", [val]);
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
  await objects.upload(bench.projectId, blob, onUpdate);
  ongoingUploads.value--;
}

async function open(blob: BlobRecord) {
  if (blob.status != BlobStatus.Available) return;
  // open blob (in new tab)
  try {
    const presignedGet = await objects.getPresignedGet(blob.id);
    window.open(presignedGet, "_blank");
  } catch (e) {
    notifications.show({
      kind: "error",
      type: "object.open",
      message: "Unable to open blob",
      description: "The blob seems to be unavailable.",
    });
    console.error(e);
  }
}

function remove(blob: BlobRecord) {
  const blobIndex = props.modelValue.findIndex((f) => f.id == blob.id);
  emit(
    "update:modelValue",
    props.modelValue.filter((f) => f.id != blob.id)
  );
  if (!isArray.value || props.modelValue.length == 1) {
    nextTick(() => uploadButtonRef.value?.focus());
  } else {
    blobRefs.focus(props.modelValue[blobIndex - 1]?.id);
  }
}

function focus() {
  if (props.modelValue.length == 0) {
    fileChooserRef.value?.click();
  } else if (isArray.value) {
    uploadButtonRef.value?.focus();
  } else {
    blobRefs.focus(props.modelValue.slice(-1)[0]?.id);
  }
}

function blur() {
  uploadButtonRef.value?.blur();
  blobRefs.refs.value.forEach((ref) => ref?.blur());
}

defineExpose({
  focus,
  blur,
  click: () => props.readonly || fileChooserRef.value?.click(),
  pending: computed(() => ongoingUploads.value > 0),
});
</script>
<template>
  <!-- Entire thing is drop zone -->
  <div
    ref="dropZoneRef"
    class="flex w-full flex-row flex-wrap gap-x-2.5 gap-y-0.5"
    :class="{
      'rounded-sm border border-dashed border-orange-500': dragOver,
      'border border-transparent': !dragOver,
      'justify-center': modelValue.length == 0,
      'min-w-[300px]': !preview,
    }"
  >
    <!-- Existing blobs -->
    <div
      :ref="(el: any) => blobRefs.registerRef(blob.id, el)"
      v-for="(blob, i) in modelValue"
      tabindex="-1"
      :key="blob.id"
      class="group/blob flex flex-row items-center rounded-sm hover:cursor-pointer focus:bg-orange-100 focus:outline-none"
      @click.stop="open(blob)"
      @keydown.enter.stop.prevent="open(blob)"
      @keydown.right.stop.prevent="
        i == modelValue.length - 1 ? uploadButtonRef?.focus() : blobRefs.focus(modelValue[i + 1]?.id)
      "
      @keydown.left.stop.prevent="i == 0 ? null : blobRefs.focus(modelValue[i - 1]?.id)"
      @keydown.delete.stop.prevent="remove(blob)"
    >
      <!-- Blob status & info -->
      <component
        :is="blob.status == BlobStatus.Uploading ? BusySpinnerIcon : DocumentArrowUpIcon"
        class="h-4 w-4 flex-shrink-0 text-gray-700"
        :class="blob.status == BlobStatus.Uploading ? 'animate-spin' : ''"
      />
      <span class="ml-1 flex flex-row items-baseline gap-1">
        <span
          class="truncate text-gray-900 underline decoration-gray-300 underline-offset-4 transition-colors duration-75 group-hover/blob:decoration-gray-700"
          >{{ blob.name }}</span
        >
        <span class="text-xs text-gray-400">{{ humanizeBytes(blob?.content_length) }}</span>
      </span>
      <!-- Delete button -->
      <button
        v-if="!preview"
        class="text-gray-300 focus:text-gray-700 group-hover/blob:text-gray-500"
        @click.stop.prevent="remove(blob)"
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
      @keydown.left.stop.prevent="blobRefs.focus(modelValue.slice(-1)[0]?.id)"
    >
      <component :is="ongoingUploads ? BusySpinnerIcon : ArrowUpTrayIcon" class="h-4 w-4 text-gray-400" />
    </button>
    <!-- Actual file chooser (unstylable, hidden) -->
    <input
      :disabled="props.readonly"
      ref="fileChooserRef"
      id="fileChooser"
      type="file"
      class="hidden"
      @change="(e) => doUpload((e.target as any)?.files?.[0])"
      @click.stop
    />
  </div>
</template>
