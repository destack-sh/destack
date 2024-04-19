<script lang="ts" setup>
import { BenchType, NodeReferenceData, NodeType, Orientation, Variant, ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline, makeIcon } from "@/system/icon";
import { isEnumType, isNodeType, toCamelName } from "@/system/lang";
import type { NodeItem } from "@/system/search";
import { enumIndex, graphIndex, useSearch, type EnumOptionItem, type SearchIndex } from "@/system/search";
import { canvas, pkgGraph } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { makeViewId } from "@/views";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const DEFAULT_WIDTH = 280;
const MAX_HEIGHT = 360;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: any } & Pick<
    ViewData,
    "title" | "text" | "icon" | "valuePacked" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);

const topicName = computed(() => {
  if (isEnumType(props.valueType?.benchType)) {
    return toCamelName(BenchType, props.valueType.benchType);
  } else if (isNodeType(props.valueType?.benchType)) {
    return toCamelName(NodeType, props.valueType.benchType);
  } else {
    return null;
  }
});
const indices: Ref<Record<string, SearchIndex<any>>> = computed(() => {
  const indices: Record<string, SearchIndex<any>> = {};
  if (isEnumType(props.valueType?.benchType)) {
    indices["enum"] = enumIndex([props.valueType.benchType]);
  } else if (isNodeType(props.valueType?.benchType)) {
    indices["graph"] = graphIndex({ graph: pkgGraph, metatypes: [props.valueType.benchType], skipDepth: 2 });
  } else {
    // TODO :Incomplete: Picker.indices
  }
  return indices;
});
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const { results, resultsTotal } = useSearch<EnumOptionItem | NodeItem>({
  query,
  indices,
  isEnabled: computed(() => props.isInline),
});

// auto-select best match when searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultId.value = results.value[0].id;
  }
});

function fire(option: EnumOptionItem | NodeItem) {
  const value = option.metatype == "enum-option" ? option.value : option.node;
  emit("update:modelValue", value);
  emit("apply", option);
}

function focus(anchor?: "previous" | "next" | FocusAnchor | NodeReferenceData) {
  const idx = results.value.findIndex((r) => r.id === activeResultId.value);
  if (anchor == "previous") {
    activeResultId.value = results.value[idx > 0 ? idx - 1 : results.value.length - 1].id;
  } else if (anchor == "next") {
    activeResultId.value = results.value[idx < results.value.length - 1 ? idx + 1 : 0].id;
  }
  if (activeResultId.value != null) {
    resultsRefs.value[activeResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
  }

  return queryRef.value;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.COMPACT], focus });
</script>
<template>
  <div>
    <!-- nocheckin: Picker variants/isInput/isInline/isDisabled/... -->
    <!-- Inline Primary: classic typeahead/combobox -->
    <div :style="{ width: DEFAULT_WIDTH + 'px' }">
      <!-- Header -->
      <div class="flex w-full flex-row items-center border-b border-gray-200 px-2.5 py-1.5">
        <IconInline v-bind="icon ?? makeIcon({ faName: 'fas fa-caret-circle-down' })" class="mr-1.5 text-gray-700" />
        <!-- Query -->
        <input
          ref="queryRef"
          type="text"
          v-model="query"
          class="w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
          :placeholder="`Select ${topicName ?? '???'}`"
          @keydown.enter.stop.prevent="activeResultId != null && fire(results.find((r) => r.id === activeResultId)!)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
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
        <ul v-if="results.length > 0" class="flex flex-col py-1">
          <template v-for="item in results" :key="item.id">
            <!-- Results -->
            <li
              :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
              role="menuitem"
              class="mx-0.5 mb-[1px] mr-1.5 mt-[1px] flex h-[28px] max-w-full flex-row items-center rounded-md border border-transparent px-2 hover:border-gray-400 hover:bg-primary-300 data-[active=true]:border-gray-400 data-[active=true]:bg-primary-300"
              :data-selected="item.id === modelValue?.id"
              :data-active="item.id === activeResultId"
              @click.prevent="fire(item)"
            >
              <!-- Content -->
              <IconInline v-if="item.icon" v-bind="item.icon" class="mr-1.5 flex-shrink-0 text-gray-700" />
              <span v-else class="mr-1.5 w-[18px] flex-shrink-0 text-gray-700" />
              <span class="select-none truncate" v-html="item.titleMarked ?? item.title" />
              <span v-if="'path' in item" class="ml-1.5 truncate text-gray-500">
                <span v-html="item.pathMarked ?? item.path" />
              </span>
              <!-- Checked -->
              <i v-if="item.id === modelValue?.id" class="fas fa-check ml-auto pl-4 pr-1 text-gray-700" />
            </li>
          </template>
        </ul>
        <!-- NOTE: Picker no results/overflow is very similar to Omnibar -->
        <!-- Too many results (truncated) -->
        <div v-if="results.length < resultsTotal" class="my-1 max-w-full px-3 pb-2 text-gray-500">
          <i class="fas fas fa-ellipsis" />
          <span class="ml-2">
            <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results for
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
  </div>
</template>
