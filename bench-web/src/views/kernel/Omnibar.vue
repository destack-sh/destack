<script lang="tsx" setup>
import { Orientation } from "@/proto/wire";
import type { SearchResult } from "@/system";
import { BUILTIN_ACTIONS, contributeAction, type ActionBuiltinId } from "@/system/action";
import { IconInline, makeIcon } from "@/system/icon";
import { ScrollbarWidth } from "@/utils/layout";
import { Casing, toCasing } from "@/utils/string";
import { Shortcut } from "@/utils/tooltip";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const PANEL_WIDTH = 600;
const PANEL_MAX_HEIGHT = 400;
const PANEL_HEADER_HEIGHT = 38;
const DEFAULT_ACTION_ICON = makeIcon({ name: "fas fas fa-arrow-right" });
// :OmnibarModes
type OmnibarMode = "everywhere" | "actions" | "space" | "views" | "page" | "module" | "package" | "bench";
const OMNIBAR_MODES: OmnibarMode[] = ["everywhere", "actions", "space", "views", "page", "module", "package", "bench"];

const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

const isActive = ref(false);
const mode = ref<OmnibarMode>("everywhere");
const query = ref<"">("");

const containerRef = ref<HTMLElement | null>(null);
const queryRef = ref<HTMLInputElement | null>(null);
const selectedResultId: Ref<string | null> = ref(null);

const candidates: Ref<SearchResult[]> = computed(() =>
  Object.values(BUILTIN_ACTIONS.value)
    .filter((a) => !a.excludeInOmnibar && (a.enabled == null || a.enabled.value))
    .map((a) => ({ ...a, metatype: "action" })),
);
const results = candidates; // nocheckin: facet search
const resultsRefs: Record<string, HTMLElement | null> = {};
const showResultCategory = true;

/** Go to the selected result */
function go() {
  if (!selectedResultId.value) throw new Error("no result selected");
  fire(selectedResultId.value);
}

async function fire(id: string) {
  const result = candidates.value.find((r) => r.id === id);
  // fire
  if (result != null) {
    if (result.metatype == "action") result.action(result);
  }
  // refocus or close
  if (id.includes(".omnibar.")) nextTick(focus);
  else close();
}

/** Select absolute/relative result */
function select(option: string | number) {
  if (typeof option === "string") {
    selectedResultId.value = option;
  } else {
    const index = results.value.findIndex((r) => r.id === selectedResultId.value);
    if (index === -1) {
      selectedResultId.value = results.value[0].id ?? null;
    } else {
      selectedResultId.value = results.value[(index + option + results.value.length) % results.value.length].id ?? null;
    }
  }
  if (selectedResultId.value != null)
    resultsRefs[selectedResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
}

function clear() {
  query.value = "";
  mode.value = "everywhere";
}

function open(inMode: OmnibarMode = "everywhere") {
  isActive.value = true;
  clear();
  mode.value = inMode;
  selectedResultId.value = candidates.value[0].id ?? null;
  nextTick(focus);
}

function focus() {
  if (document.activeElement != queryRef.value && isActive.value) {
    queryRef.value!.focus();
  }
}

function close() {
  isActive.value = false;
  clear();
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

// actions (assumes Omnibar is a singleton, also see :OmnibarModes)
const SHORTCUTS_BY_MODE: Partial<Record<OmnibarMode, string[]>> = {
  everywhere: ["mod+k"],
  actions: ["mod+shift+a"],
  space: ["mod+shift+s"],
  views: ["mod+shift+v"],
  page: ["mod+p"],
  module: ["mod+shift+m"],
  package: ["mod+shift+p"],
  bench: ["mod+shift+b"],
};
const TEXT_BY_MODE: Record<OmnibarMode, string> = {
  everywhere: "Search anything",
  actions: "Find an action to run",
  space: "Search across your Space",
  views: "Search Views in your Space",
  page: "Search the current page (Block)",
  module: "Search the current Module",
  package: "Search the current Package",
  bench: "Search the current Bench",
};
for (const inMode of OMNIBAR_MODES) {
  contributeAction({
    id: ("space.open.omnibar." + inMode) as ActionBuiltinId,
    title: `Search ${toCasing(inMode, Casing.CAMEL)}`,
    shortcuts: SHORTCUTS_BY_MODE[inMode] ?? [],
    icon: inMode == "actions" ? "fas fa-command" : "fas fa-magnifying-glass",
    text: TEXT_BY_MODE[inMode],
    action: () => open(inMode),
    enabled: computed(() => props.box.width >= PANEL_WIDTH && (!isActive.value || inMode != mode.value)),
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
    leave-active-class="transition-all ease-out duration-75"
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
        enter-active-class="transition-all ease-in duration-75"
        enter-from-class="scale-95"
        enter-to-class="scale-100"
        appear
      >
        <div
          v-if="isActive /* trigger inner transition */"
          ref="containerRef"
          class="z-60 fixed h-fit rounded-md border border-gray-700 bg-white text-sm opacity-100 shadow-md shadow-gray-700 transition-transform duration-150"
          :style="{
            top: box.top + 'px',
            width: PANEL_WIDTH + 'px',
            maxHeight: PANEL_MAX_HEIGHT + 'px',
            left: box.left + box.width / 2 - PANEL_WIDTH / 2 + 'px',
          }"
          @click.stop.prevent="focus"
        >
          <!-- Header (pr is +2px for inset scroll track) -->
          <div
            class="flex w-full flex-row items-center gap-x-2 border-b border-gray-700 pl-4 pr-[18px] text-sm text-gray-900"
            :style="{ height: PANEL_HEADER_HEIGHT + 'px' }"
          >
            <!-- Icon -->
            <i class="fas fa-magnifying-glass text-gray-500" />
            <!-- Mode -->
            <span v-if="mode != 'everywhere'" class="select-none font-semibold">
              {{ toCasing(mode, Casing.CAMEL) }}
            </span>
            <!-- Query -->
            <input
              ref="queryRef"
              type="text"
              v-model="query"
              :placeholder="TEXT_BY_MODE[mode] + '...'"
              class="h-full w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
              @keydown.enter.stop.prevent="go"
              @keydown.down.stop.prevent="select(1)"
              @keydown.up.stop.prevent="select(-1)"
            />
            <!-- Close -->
            <button class="ml-auto" @click="close">
              <Shortcut shortcut="esc" />
            </button>
          </div>

          <!-- Body -->
          <Scroll
            :orientation="Orientation.VERTICAL"
            track-is-always-visible
            :trackWidth="ScrollbarWidth.sm"
            :size="{
              width: PANEL_WIDTH,
              height: PANEL_MAX_HEIGHT - PANEL_HEADER_HEIGHT - 2 /* border */,
            }"
          >
            <!-- Results -->
            <ul class="mb-0.5 flex w-full select-none flex-col px-2 py-1 text-gray-900">
              <template v-for="(result, i) in results" :key="result.id">
                <!-- Category -->
                <div
                  v-if="showResultCategory && (i === 0 || results[i - 1].category !== result.category)"
                  class="-mx-2 mb-0.5 px-4 pt-1"
                  :class="[i > 0 ? 'mt-1.5 border-t border-gray-900' : '']"
                >
                  <span class="text-xs font-semibold text-gray-500">
                    {{ toCasing(result.category, Casing.CAMEL) }}
                  </span>
                </div>
                <!-- Result -->
                <li
                  :ref="(ref: any | undefined) => (ref != null ? (resultsRefs[result.id] = ref) : delete resultsRefs[result.id])"
                  role="button"
                  :data-selected="result.id === selectedResultId"
                  :class="[
                    'text my-0.5 flex w-full flex-row items-center rounded-md border border-transparent px-2 py-1',
                    'hover:bg-primary-300 data-[selected=true]:border-gray-900 data-[selected=true]:bg-primary-300',
                  ]"
                  @click.stop.prevent="() => fire(result.id)"
                >
                  <IconInline v-bind="result.icon ?? DEFAULT_ACTION_ICON" class="text-gray-600" />
                  <span class="ml-2">{{ result.title }}</span>
                  <!-- Shortcut -->
                  <Shortcut
                    v-if="(result.shortcuts?.length ?? 0) > 0"
                    class="ml-auto"
                    :shortcut="result.shortcuts![0]"
                  />
                </li>
              </template>
            </ul>
          </Scroll>
        </div>
      </Transition>
    </div>
  </Transition>
</template>
