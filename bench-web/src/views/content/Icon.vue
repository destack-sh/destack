<script lang="ts" setup>
import { ViewData, NodeType, IconData, NodeReferenceData, Orientation, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef, watch, type Ref } from "vue";
import { makeViewId } from "@/views";
import Scroll from "@/views/containers/Scroll.vue";
import { IconInline, getIconMetadata, makeIcon, metadataToIcon, type IconMetadata } from "@/system/icon";
import { ScrollbarWidth } from "@/utils/layout";
import { iconIndex, useSearch, type IconItem, type SearchIndex } from "@/system/search";
import type { TooltipInfo } from "@/utils/tooltip";
import type { OverlayMenuInfo, OverlayMenuInfoIn } from "@/utils/menu";

const DEFAULT_WIDTH = 380;
const MAX_HEIGHT = 280;
const ITEMS_PER_ROW = 10;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: IconData } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);
const indices: Ref<Record<string, SearchIndex<any>>> = computed(() => {
  const indices: Record<string, SearchIndex<any>> = {};
  indices["icon"] = iconIndex();
  return indices;
});
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const { results, resultsTotal } = useSearch<IconItem>({
  query,
  indices,
  isEnabled: computed(() => props.isInline),
  options: { outOfOrder: 0, highlight: false, maxResults: 32 * ITEMS_PER_ROW },
});

// auto-select best match when searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultId.value = results.value[0].id;
  }
});

function fire(item: IconMetadata) {
  const icon = metadataToIcon(item);
  apply(icon);
}
function apply(icon: IconData) {
  emit("update:modelValue", icon);
  emit("apply", icon);
}

function focus(anchor?: "up" | "down" | "left" | "right" | FocusAnchor | NodeReferenceData) {
  let nextIdx;
  const currentIdx = results.value.findIndex((item) => item.id === activeResultId.value);
  if (anchor === "up") {
    nextIdx = currentIdx - ITEMS_PER_ROW;
  } else if (anchor === "down") {
    nextIdx = currentIdx + ITEMS_PER_ROW;
  } else if (anchor === "left") {
    nextIdx = currentIdx - 1;
  } else if (anchor === "right") {
    nextIdx = currentIdx + 1;
  } else {
    nextIdx = 0;
  }
  nextIdx = Math.max(0, Math.min(results.value.length - 1, nextIdx));
  activeResultId.value = results.value[nextIdx].id;
  if (resultsRefs.value[activeResultId.value] != null) {
    resultsRefs.value[activeResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
  }
  queryRef.value?.focus();
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- TODO :Incomplete: Icon.isInput/isDisabled/variants/... -->
    <!-- Dropdown -->
    <button
      v-if="!isInline"
      class="group flex w-full items-center rounded border border-gray-200 px-2 py-1 hover:border-gray-400 data-[menu=true]:border-gray-400"
      v-menu="
        (): OverlayMenuInfoIn => ({
          component: ViewType.ICON,
          placement: 'bottom-left',
          offset: 'referenceWidth',
          props: { ...props, isInline: true },
          onApply: (value) => apply(value),
        })
      "
    >
      <template v-if="icon != null">
        <IconInline v-bind="icon" class="" />
        <span class="ml-1.5">{{ getIconMetadata(icon)?.title ?? "Custom Icon" }}</span>
      </template>
      <template v-else>
        <i class="fas fa-icons text-gray-400 group-hover:text-gray-700" />
        <span class="ml-1.5 text-gray-400 group-hover:text-gray-700">Select Icon</span>
      </template>
      <i class="fas fa-caret-down ml-auto pl-1.5 text-gray-400" />
    </button>

    <!-- Inline Combobox -->
    <div v-else-if="isInline" :style="{ width: DEFAULT_WIDTH + 'px' }">
      <!-- Header -->
      <div class="flex w-full flex-row items-center border-b border-gray-200 px-2.5 py-1.5">
        <IconInline v-bind="icon ?? makeIcon({ faName: 'fas fa-magnifying-glass' })" class="mr-1.5 text-gray-700" />
        <!-- Query -->
        <input
          ref="queryRef"
          type="text"
          v-model="query"
          class="w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
          :placeholder="`Search Icons...`"
          @keydown.enter.stop.prevent="activeResultId != null && fire(results.find((r) => r.id === activeResultId)!)"
          @keydown.up.stop.prevent="focus('up')"
          @keydown.down.stop.prevent="focus('down')"
          @keydown.left.stop.prevent="focus('left')"
          @keydown.right.stop.prevent="focus('right')"
        />
      </div>
      <!-- Body -->
      <Scroll
        size-is-dynamic
        :size="{ width: DEFAULT_WIDTH, height: MAX_HEIGHT }"
        :orientation="Orientation.VERTICAL"
        :track-width="ScrollbarWidth.sm"
        track-is-overlay
      >
        <!-- Results -->
        <ul v-if="results.length > 0" class="grid grid-cols-10 gap-x-1 gap-y-1 px-2 py-2">
          <template v-for="(item, i) in results" :key="i">
            <span
              :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
              class="select-none rounded border border-transparent py-1.5 text-center text-gray-700 hover:cursor-pointer hover:border-gray-300 hover:bg-primary-200 hover:text-gray-900 data-[active=true]:border-gray-300 data-[active=true]:bg-primary-200"
              :class="item.faName"
              role="menuitem"
              :data-selected="item.faName == modelValue?.faName"
              :data-active="item.id === activeResultId"
              v-tooltip="
                {
                  isEnabled: results[i] != null,
                  title: () => results[i]?.title /* results[i] because 'item' is captured once and never updated */,
                  showDelay: 200,
                  hideDelay: 100,
                  small: true,
                } as TooltipInfo
              "
              @click.stop.prevent="fire(item)"
            />
          </template>
        </ul>
        <!-- NOTE: Picker no results/overflow is very similar to Omnibar/Picker/etc :ResultInfo -->
        <!-- Too many results (truncated) -->
        <div v-if="results.length < resultsTotal" class="my-1 max-w-full px-3 pb-2 text-gray-500">
          <i class="fas fas fa-ellipsis" />
          <span class="ml-2">
            <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results
            <template v-if="query.length > 0">for </template>
            <span class="truncate font-semibold">{{ query }}</span>
            (showing {{ results.length }})
          </span>
        </div>
        <!-- Help -->
        <div v-if="results.length == 0" class="max-w-full px-1 py-1">
          <!-- Nothing found -->
          <div v-if="results.length === 0" class="px-2 py-1 text-gray-500">
            <i class="fas fa-empty-set text-gray-600" />
            <span class="ml-1">
              No results
              <span v-if="query">
                for <span class="truncate font-semibold">{{ query }}</span>
              </span>
            </span>
          </div>
        </div>
      </Scroll>
    </div>
  </ViewContentWrapper>
</template>
