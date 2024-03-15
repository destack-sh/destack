<script lang="tsx" setup>
import PlainText from "@/views/content/PlainText.vue";
import { ref, watch } from "vue";

const PANEL_WIDTH = 600;
const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

const isActive = ref(false);
const mode = ref<"">("");
const query = ref<"">("");

const containerRef = ref<HTMLElement | null>(null);

function show() {
  isActive.value = true;
  // TODO: focus
}

// auto-close when the box is too small
watch(
  () => props.box.width,
  () => {
    if (props.box.width < PANEL_WIDTH && isActive.value) {
      isActive.value = false;
    }
  },
);

defineExpose({ isActive, show });
</script>
<template>
  <!-- Backdrop -->
  <Transition
    enter-active-class="transition-opacity ease-in duration-75"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-active-class="transition-opacity ease-out duration-100"
    leave-from-class="opacity-100"
    leave-to-class="opacity-0"
    appear
  >
    <div
      v-if="isActive"
      class="fixed left-0 top-0 z-50 flex h-screen w-screen justify-center bg-gray-600 bg-opacity-20"
      @keydown.esc.exact.prevent="isActive = false"
      @click="isActive = false"
    >
      <!-- Modal -->
      <Transition
        enter-active-class="transition-all ease-in duration-100"
        enter-from-class="scale-95"
        enter-to-class="scale-100"
        leave-active-class="transition-all ease-out duration-100"
        leave-from-class="scale-100 translate-y-0"
        leave-to-class="translate-y-[-10px]"
        appear
      >
        <div
          ref="containerRef"
          class="z-60 fixed h-fit rounded-md border border-gray-900 bg-white py-3 text-lg opacity-100 shadow-md shadow-gray-900"
          :style="{
            top: box.top + 'px',
            width: PANEL_WIDTH + 'px',
            left: box.left + box.width / 2 - PANEL_WIDTH / 2 + 'px',
          }"
          @click.stop.prevent
        >
          <!-- Header -->
          <div class="flex h-8 w-full flex-row items-center border-b border-gray-900 px-4">
            
            <!-- Query -->
            <input
              type="text"
              v-model="query"
              class="h-full w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
            />
          </div>
          <!-- Body -->
          <div class="px-4 py-2">
            <span> Results... </span>
          </div>
        </div>
      </Transition>
    </div>
  </Transition>
</template>
