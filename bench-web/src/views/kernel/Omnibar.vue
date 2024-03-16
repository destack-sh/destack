<script lang="tsx" setup>
import { BUILTIN_ACTIONS, contributeAction, type ActionBuiltinId } from "@/system/action";
import { IconInline, makeIcon } from "@/system/icon";
import { ScrollbarWidth } from "@/utils/layout";
import { Casing, toCasing } from "@/utils/string";
import { Shortcut } from "@/utils/tooltip";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const PANEL_WIDTH = 600;
const PANEL_MAX_HEIGHT = 600;
const PANEL_HEADER_HEIGHT = 42;
const DEFAULT_ACTION_ICON = makeIcon({ name: "fas fas fa-arrow-right" });
type OmnibarMode = "universal" | "action" | "search" | "space" | "package" | "bench";
const OMNIBAR_MODES: OmnibarMode[] = ["universal", "action", "search", "space", "package", "bench"];

const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

const isActive = ref(false);
const mode = ref<OmnibarMode>("universal");
const query = ref<"">("");
const placeholder = computed(() => "Search or jump to...");

const containerRef = ref<HTMLElement | null>(null);
const queryRef = ref<HTMLInputElement | null>(null);
const selectedResultId: Ref<string | null> = ref(null);

const candidates = computed(() =>
  Object.values(BUILTIN_ACTIONS.value).filter((a) => !a.excludeInOmnibar && (a.enabled == null || a.enabled.value)),
);
const results = candidates; // TODO :Broken: filter actions/search/etc

/** Go to the selected result */
function go() {
  if (!selectedResultId.value) throw new Error("no result selected");
  // TODO :Broken: do result thing
  close();
}

/** Select absolute/relative result */
function select(option: string | number) {
  if (typeof option === "string") {
    selectedResultId.value = option;
  } else {
    const index = results.value.findIndex((r) => r.key === selectedResultId.value);
    if (index === -1) {
      selectedResultId.value = results.value[0].key;
    } else {
      selectedResultId.value = results.value[(index + option + results.value.length) % results.value.length].key;
    }
  }
}

function open(mode: OmnibarMode = "universal") {
  isActive.value = true;
  query.value = "";
  nextTick(() => queryRef.value!.focus());
}

function close() {
  isActive.value = false;
  query.value = "";
}

// auto-close when the box becomes too small
watch(
  () => props.box.width,
  () => {
    if (props.box.width < PANEL_WIDTH && isActive.value) {
      isActive.value = false;
    }
  },
);

// actions (assumes singleton)
const SHORTCUTS_BY_MODE: Record<OmnibarMode, string[]> = {
  universal: ["mod+k"],
  action: ["mod+shift+a"],
  search: ["mod+shift+s"],
  space: ["mod+shift+v"],
  package: ["mod+shift+p"],
  bench: ["mod+shift+b"],
};
const TEXT_BY_MODE: Record<OmnibarMode, string> = {
  universal: "Search across everything",
  action: "Run an action",
  search: "Search across your Space",
  space: "Search views in your Space",
  package: "Search the current Backage",
  bench: "Search the current Bench",
};
for (const mode of OMNIBAR_MODES) {
  contributeAction({
    id: ("space.open.omnibar." + mode) as ActionBuiltinId,
    title: `Open ${toCasing(mode, Casing.CAMEL)} Omnibar`,
    shortcuts: SHORTCUTS_BY_MODE[mode],
    icon: mode == "action" ? "fas fa-command" : "fas fa-magnifying-glass",
    text: TEXT_BY_MODE[mode],
    action: () => open(mode),
    excludeInOmnibar: true,
  });
}

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
            maxHeight: PANEL_MAX_HEIGHT + 'px',
            left: box.left + box.width / 2 - PANEL_WIDTH / 2 + 'px',
          }"
          @click.stop.prevent
        >
          <!-- Header -->
          <div
            class="flex w-full flex-row items-center gap-x-2 border-b border-gray-700 px-4 py-2.5 text-gray-900"
            :style="{ height: PANEL_HEADER_HEIGHT + 'px' }"
          >
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
              @keydown.enter.stop.prevent="go"
              @keydown.down.stop.prevent="select(1)"
              @keydown.up.stop.prevent="select(-1)"
            />
          </div>

          <!-- Body -->
          <Scroll
            :trackWidth="ScrollbarWidth.sm"
            track-is-overlay
            size-is-dynamic
            :size="{
              width: PANEL_WIDTH,
              height: PANEL_MAX_HEIGHT - PANEL_HEADER_HEIGHT,
            }"
          >
            <div class="px-2 py-2 text-gray-900">
              <!-- Results -->
              <ul class="flex w-full flex-col gap-y-0.5">
                <template v-for="result in results" :key="result.key">
                  <!-- Result -->
                  <li
                    role="button"
                    :data-selected="result.key === selectedResultId"
                    :class="[
                      'flex w-full flex-row items-center rounded-md border border-transparent px-2.5 py-0.5 transition-colors duration-100',
                      'hover:border-gray-700 hover:bg-primary-300 data-[selected=true]:border-gray-700 data-[selected=true]:bg-primary-300',
                    ]"
                    @click="() => result.action()"
                  >
                    <IconInline v-bind="result.icon ?? DEFAULT_ACTION_ICON" />
                    <span class="ml-2">{{ result.title }}</span>
                    <!-- Shortcut -->
                    <Shortcut v-if="(result.shortcuts?.length ?? 0) > 0" :shortcut="result.shortcuts![0]" />
                  </li>
                </template>
              </ul>
            </div>
          </Scroll>
        </div>
      </Transition>
    </div>
  </Transition>
</template>
