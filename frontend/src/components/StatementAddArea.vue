<script lang="ts" setup>
import { useMagicActions, useNavigationContext } from "@/components/file";
import { useRelativeDropZone } from "@/utils/drop";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { ref } from "vue";

const props = defineProps<{ position: "start" | "end" }>();
const buttonRef = ref(null);
const { isOverDropZone } = useRelativeDropZone(buttonRef, ["File"], onDrop);
const magic = useMagicActions(ref(null));
const nav = useNavigationContext();

function onDrop(files: File[] | any) {
  if (Array.isArray(files)) {
    const location = props.position == "start" ? nav.value.getLocationStart() : nav.value.getLocationEnd();
    magic.insertFilesAsDataset(location, files);
  }
}
</script>
<template>
  <button
    ref="buttonRef"
    class="group relative flex cursor-default py-1 opacity-0 outline-none transition duration-150 hover:opacity-100"
    :class="isOverDropZone ? 'opacity-100' : 'opacity-0'"
  >
    <!-- Drag indicators (bottom if start, top if end) -->
    <div
      v-if="position == 'end'"
      class="absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
    />
    <div
      v-if="position == 'start'"
      class="absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
    />
    <div class="justify-left relative flex align-top">
      <span class="rounded-sm bg-white p-0.5 px-2 text-gray-500 hover:bg-orange-100">
        <PlusIcon class="h-4 w-4" aria-hidden="true" />
      </span>
    </div>
  </button>
</template>
