<script lang="tsx" setup>
import { BUILTIN_ACTIONS } from "@/system/action";
import { IconInline, makeIcon } from "@/system/icon";
import { computed, nextTick, ref, watch } from "vue";

const PANEL_WIDTH = 600;
const DEFAULT_ACTION_ICON = makeIcon({ name: "fas fas fa-arrow-right" });

const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

const isActive = ref(false);
type ActionPaletteMode = "universal" | "action" | "space" | "package" | "bench";
const mode = ref<ActionPaletteMode>("universal");
const query = ref<"">("");
const placeholder = computed(() => "Search or jump to...");

const containerRef = ref<HTMLElement | null>(null);
const queryRef = ref<HTMLInputElement | null>(null);

function open(mode: ActionPaletteMode = "universal") {
  isActive.value = true;
  query.value = "";
  nextTick(() => queryRef.value!.focus());
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

defineExpose({ isActive, open });
</script>
<template>
  <!-- Backdrop -->
  <Transition
    enter-active-class="transition-opacity ease-in duration-75"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-active-class="transition-all ease-out duration-100"
    leave-from-class="opacity-100 translate-y-0"
    leave-to-class="opacity-0 translate-y-[-10px]"
    appear
  >
    <div
      v-if="isActive"
      class="fixed left-0 top-0 z-50 flex h-screen w-screen justify-center bg-gray-700 bg-opacity-20"
      @keydown.esc.exact.prevent="isActive = false"
      @click="isActive = false"
    >
      <!-- Modal -->
      <Transition
        enter-active-class="transition-all ease-in duration-100"
        enter-from-class="scale-95"
        enter-to-class="scale-100"
        appear
      >
        <div
          v-if="isActive /* trigger inner transition */"
          ref="containerRef"
          class="z-60 fixed h-fit rounded-md border border-gray-700 bg-white text-lg opacity-100 shadow-md shadow-gray-700"
          :style="{
            top: box.top + 'px',
            width: PANEL_WIDTH + 'px',
            left: box.left + box.width / 2 - PANEL_WIDTH / 2 + 'px',
          }"
          @click.stop.prevent
        >
          <!-- Header -->
          <div class="flex w-full flex-row items-center gap-x-2 border-b border-gray-700 px-4 py-2.5 text-gray-900">
            <!-- Icon -->
            <i class="fas fa-magnifying-glass text-gray-400" />
            <!-- Mode -->
            <span v-if="mode != 'universal'">
              {{ mode }}
            </span>
            <!-- Query -->
            <input
              ref="queryRef"
              type="text"
              v-model="query"
              :placeholder="placeholder"
              class="h-full w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
            />
          </div>
          <!-- Body -->
          <div class="px-2 py-2.5 text-gray-900">
            <!-- Results -->
            <ul class="flex w-full flex-col gap-y-0.5">
              <template v-for="result in Object.values(BUILTIN_ACTIONS)" :key="result.key">
                <!-- Result -->
                <li
                  role="button"
                  class="flex w-full flex-row items-center rounded-md px-2.5 py-0.5 hover:bg-primary-300"
                  @click="() => result.action()"
                >
                  <IconInline v-bind="result.icon ?? DEFAULT_ACTION_ICON" />
                  <span class="ml-2">{{ result.title }}</span>
                  <!--  -->
                </li>
              </template>
            </ul>
          </div>
        </div>
      </Transition>
    </div>
  </Transition>
</template>
