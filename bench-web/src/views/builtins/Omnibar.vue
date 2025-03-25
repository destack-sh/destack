<script lang="tsx" setup>
import { INLINE_NODE_TYPES } from "@/language/core/const";
import { NodeMode, NodeType, ObjectType, Orientation } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { packagePtr } from "@/system/client";
import { canvas, hasLocalPkg, benchGraph } from "@/system/space";
import { OMNIBAR_MODES, addCommand, fireCommand, type CommandBuiltinId, type OmnibarMode } from "@/ui/command";
import { IconInline, makeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import {
  COMMAND_INDEX,
  graphIndex,
  isHiddenBuiltinNodeItem,
  useIndexSearch,
  type CommandItem,
  type NodeItem,
  type SearchIndex,
} from "@/ui/search";
import { Shortcut } from "@/ui/tooltip";
import { nowOrNextTick } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const PANEL_WIDTH = 600;
const PANEL_MAX_HEIGHT = 420;
const PANEL_HEADER_HEIGHT = 44;
const DEFAULT_COMMAND_ICON = makeIcon({ faName: "fas fas fa-arrow-right" });

const props = defineProps<{ box: { left: number; top: number; width: number; height: number } }>();

const isActive = ref(false);
const mode = ref<OmnibarMode>("bench");
const query = ref<"">("");

const containerRef = ref<HTMLElement | null>(null);
const queryRef = ref<HTMLInputElement | null>(null);
const activeResultLocalId: Ref<string | null> = ref(null);

const isQueryEmpty = computed(() => query.value.length === 0);
const indices = computed(() => {
  const indices: Record<string, SearchIndex<any>> = {};

  // commands
  if (["bench", "commands"].includes(mode.value)) {
    indices["Commands"] = COMMAND_INDEX;
  }

  // package
  if (packagePtr.value != null && hasLocalPkg.value && ["bench"].includes(mode.value)) {
    indices["Bench"] = graphIndex({
      id: "bench",
      graph: benchGraph,
      metatypes: [...INLINE_NODE_TYPES],
      roots: [benchGraph.getOrError(packagePtr.value)],
      skipDepth: 1,
      // only search deeply if in bench search specifically
      maxDepth: isQueryEmpty.value && mode.value != "bench" ? 1 : undefined,
      filter: (node, ancestors) => {
        if (isHiddenBuiltinNodeItem(node)) {
          return false; // :HiddenBuiltinStuff
        } else if (node.metatype == ObjectType.VIEW && ancestors.some((a) => a.node.metatype == ObjectType.SPACE)) {
          return false;
        } else {
          return true;
        }
      },
    });
  }

  return indices;
});
const { candidates, results, resultsTotal, updateCandidates } = useIndexSearch<NodeItem | CommandItem>({
  query,
  isEnabled: isActive,
  indices,
});
const resultsRefs: Record<string, HTMLElement | null> = {};
const showResultCategory = computed(() => query.value.length === 0);

/** Go to the selected result */
function go() {
  if (!activeResultLocalId.value) throw new Error("no result selected");
  fire(activeResultLocalId.value);
}

/** Fires the command associated with the given result  */
async function fire(id: string) {
  const result = candidates.value.find((r) => r.itemId === id);
  // fire
  if (result != null) {
    if (result.metatype == "command") fireCommand(result);
    else if (result.metatype == "node") canvas.goToNode(result.node);
    else throw new Error(`unexpected result: ${result}`);
  }
  // refocus or close
  if (id.includes(".omnibar.")) nextTick(focus);
  else close({ delayFocus: true }); // command may have just created a new view
}

/** Select absolute/relative result */
function select(option: string | number | null) {
  if (typeof option === "string" || option == null) {
    activeResultLocalId.value = option;
  } else {
    const index = results.value.findIndex((r) => r.itemId === activeResultLocalId.value);
    if (index === -1) {
      activeResultLocalId.value = results.value[0].itemId ?? null;
    } else {
      activeResultLocalId.value =
        results.value[(index + option + results.value.length) % results.value.length].itemId ?? null;
    }
  }
  if (activeResultLocalId.value != null) {
    resultsRefs[activeResultLocalId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
  }
}

// auto-select best match while searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultLocalId.value = results.value[0].itemId;
  }
});

function clear() {
  query.value = "";
  mode.value = "bench";
}

function open(inMode: OmnibarMode = "bench") {
  isActive.value = true;
  clear();
  mode.value = inMode;
  updateCandidates();
  select(candidates.value[0]?.itemId ?? null);
  nextTick(focus);
}

function focus() {
  if (document.activeElement != queryRef.value && isActive.value) {
    if (queryRef.value == null) throw new Error(`cannot focus Omnibar: queryRef is ${queryRef.value}`);
    else queryRef.value!.focus();
  }
}

function close(options?: { delayFocus: boolean }) {
  isActive.value = false;
  clear();
  nowOrNextTick(options?.delayFocus, () => canvas.restoreComponentFocus());
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

//
// Commands (assumes Omnibar is a singleton, also see :OmnibarModes)
//

const SHORTCUTS_BY_MODE: Partial<Record<OmnibarMode, string[]>> = {
  bench: ["mod+k"],
  commands: ["ctrl+k"],
};
const TEXT_BY_MODE: Record<OmnibarMode, string> = {
  bench: "Search across Bench",
  commands: "Find a command to run",
};

for (const inMode of OMNIBAR_MODES) {
  addCommand("static", {
    id: ("space.omnibar." + inMode) as CommandBuiltinId,
    title: `Search ${toCasing(inMode, Casing.CAMEL)}`,
    shortcuts: SHORTCUTS_BY_MODE[inMode] ?? [],
    icon: inMode == "commands" ? "fas fa-command" : "fas fa-magnifying-glass",
    text: TEXT_BY_MODE[inMode],
    command: () => open(inMode),
    isEnabled: computed(() => props.box.width >= PANEL_WIDTH),
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
      data-outside-view="true"
      @keydown.esc.exact.prevent="() => close()"
      @click.stop.prevent="isActive = false"
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
          class="fixed z-60 h-fit rounded border border-gray-400 bg-white text-sm opacity-100 transition-transform duration-150"
          :style="{
            top: box.top + 40 + 'px',
            width: PANEL_WIDTH + 'px',
            maxHeight: PANEL_MAX_HEIGHT + 'px',
            left: box.left + box.width / 2 - PANEL_WIDTH / 2 + 'px',
          }"
          @click.stop.prevent="focus"
        >
          <!-- Header (pr is +2px for inset scroll track) -->
          <div
            class="flex w-full flex-row items-center gap-x-2 border-b border-gray-200 pl-[18px] pr-[18px] text-sm text-gray-900"
            :style="{ height: PANEL_HEADER_HEIGHT + 'px' }"
          >
            <!-- Icon -->
            <i class="fas fa-magnifying-glass w-5 text-center text-base text-gray-500" />
            <!-- Mode -->
            <span v-if="mode != 'bench'" class="select-none font-semibold">
              {{ toCasing(mode, Casing.CAMEL) }}
            </span>
            <!-- Query -->
            <input
              ref="queryRef"
              v-model="query"
              type="text"
              :placeholder="TEXT_BY_MODE[mode] + '...'"
              class="h-full w-full border-0 bg-transparent p-0 text-base placeholder-gray-500 outline-none ring-0 focus:ring-0"
              @keydown.enter.stop.prevent="go"
              @keydown.down.stop.prevent="select(1)"
              @keydown.up.stop.prevent="select(-1)"
              @keydown.delete="query.length > 0 || (mode = 'bench')"
            />
            <!-- Close -->
            <button class="ml-auto" @click="() => close()">
              <Shortcut class="text-gray-700" shortcut="esc" />
            </button>
          </div>

          <!-- Body -->
          <Scroll
            id="scroll"
            :orientation="Orientation.VERTICAL"
            :track-width="ScrollbarWidth.sm"
            size-is-dynamic
            :size="{
              width: PANEL_WIDTH,
              height: PANEL_MAX_HEIGHT - PANEL_HEADER_HEIGHT - 2 /* border */,
            }"
          >
            <!-- Results -->
            <ul v-if="results.length > 0" class="flex w-full select-none flex-col px-2 py-1 text-gray-900">
              <template v-for="(item, i) in results" :key="i">
                <!-- Category -->
                <div
                  v-if="showResultCategory && (i === 0 || results[i - 1].category !== item.category)"
                  class="-mx-2 mb-0.5 px-4 pt-0.5"
                  :class="[i > 0 ? 'mt-1' : '']"
                >
                  <span class="text-xs font-semibold text-gray-500">
                    {{ toCasing(item.category, Casing.CAMEL) }}
                  </span>
                </div>
                <!-- Result -->
                <li
                  :ref="(ref: any | undefined) => (ref != null ? (resultsRefs[item.itemId] = ref) : delete resultsRefs[item.itemId])"
                  role="button"
                  :data-selected="item.itemId === activeResultLocalId"
                  class="my-0.5 flex w-full flex-row items-center rounded border border-transparent px-2 py-1 transition-colors duration-75 hover:bg-gray-100 data-[selected=true]:bg-gray-100"
                  @click.stop.prevent="() => fire(item.itemId)"
                >
                  <!-- Content -->
                  <IconInline
                    v-bind="item.icon ?? DEFAULT_COMMAND_ICON"
                    class="w-5"
                    :class="item.itemId == activeResultLocalId ? '' : 'text-gray-700'"
                  />
                  <!-- Title/Path -->
                  <span class="ml-2 truncate">
                    <!-- Title -->
                    <span v-html="item.titleMarked ?? item.title" />
                    <!-- Path -->
                    <span class="ml-2 text-gray-500">
                      <span v-html="item.pathMarked ?? item.path" />
                    </span>
                  </span>
                  <!-- Metadata -->
                  <NodeMetadata v-if="item.metatype == 'node'" size="sm" :node="item.node!" class="ml-1.5" />
                  <!-- Secondary (shortcut, last edited, etc.) -->
                  <span class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-2">
                    <Shortcut
                      v-if="item.metatype == 'command' && (item.shortcuts?.length ?? 0) > 0"
                      class="text-gray-700"
                      :shortcut="item.shortcuts![0]"
                    />
                    <span v-if="!showResultCategory" class="text-gray-500">{{ item.index }}</span>
                  </span>
                </li>
              </template>
            </ul>
            <!-- Too many results (truncated) -->
            <div v-if="results.length < resultsTotal" class="my-1 px-[18px] pb-2 text-gray-500">
              <i class="fas fas fa-ellipsis w-5 text-center" />
              <span class="ml-2">
                <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results
                <template v-if="query.length > 0"> for</template>
                <span class="font-semibold">{{ query }}</span>
                (showing {{ results.length }})
              </span>
            </div>
            <!-- Help -->
            <!-- NOTE: the horizontal spacing of 'too many' and 'no results' is intentionally different
               to align with the results & input respectively -->
            <div v-else-if="results.length == 0" class="my-1 px-2 py-1">
              <!-- Nothing found -->
              <div v-if="results.length === 0" class="px-2.5 py-1 text-gray-500">
                <i class="fas fa-empty-set w-5 text-center text-gray-600" />
                <span class="ml-1">
                  No results
                  <span v-if="query">
                    for <span class="font-semibold">{{ query }}</span>
                  </span>
                </span>
              </div>
            </div>
          </Scroll>
        </div>
      </Transition>
    </div>
  </Transition>
</template>
