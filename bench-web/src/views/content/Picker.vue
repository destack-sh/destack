<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType, BenchType, Variant, Orientation } from "@/proto/wire";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import { canvas, pkgGraph } from "@/system/space";
import { ref, toRef, type Ref, computed } from "vue";
import { makeViewId } from "@/views";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { getEnumOptions, toCamelName, type EnumOption, isEnumType, isNodeType } from "@/system/lang";
import { EnumType } from "@/proto/wire";
import { enumIndex, graphIndex, type EnumOptionItem, type SearchIndex, useSearch } from "@/system/search";
import type { NodeItem } from "@/system/search";
import Scroll from "@/views/containers/Scroll.vue";
import { ScrollbarWidth } from "@/utils/layout";
import { IconInline } from "@/system/icon";

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
  if (isEnumType(props.valueType?.benchType)) {
    return { enum: enumIndex([props.valueType.benchType]) };
  } else if (isNodeType(props.valueType?.benchType)) {
    return { graph: graphIndex({ graph: pkgGraph, metatypes: [props.valueType.benchType] }) };
  } else {
    // TODO :Incomplete: Picker.indices
    return {} as Record<string, SearchIndex<any>>;
  }
});
const options = getEnumOptions(EnumType.BLOCK_TYPE); // nocheckin: get generic options (use SearchIndex)
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const { candidates, results, resultsTotal, updateCandidates } = useSearch<EnumOptionItem | NodeItem>({
  query,
  indices,
  isEnabled: computed(() => props.isInline),
});

//
// Interaction
//

function fire(option: EnumOptionItem | NodeItem) {
  emit("update:modelValue", option);
  emit("apply", option);
}

function focus(anchor?: "previous" | "next" | FocusAnchor | NodeReferenceData) {
  // nocheckin: interaction
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
  <!-- nocheckin: Picker variants/isInput/isInline/isDisabled/... -->
  <div>
    <div :style="{ width: DEFAULT_WIDTH + 'px' }">
      <!-- Header -->
      <div class="w-full border-b border-gray-200 px-3 py-1.5">
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
        <ul class="flex flex-col py-1">
          <template v-for="item in results" :key="item.id">
            <!-- Results -->
            <li
              :ref="(ref?: any) => ref != null ? (resultsRefs[item.id] = ref) : (delete resultsRefs[item.id])"
              role="menuitem"
              class="mx-1 mb-[1px] mt-[2px] flex h-[28px] w-full max-w-full flex-row items-center rounded-md border border-transparent px-2 hover:border-gray-400 hover:bg-primary-300 data-[active=true]:bg-primary-300"
              :data-selected="item.id === modelValue?.id"
              :data-active="item.id === activeResultId"
              @click.prevent="fire(item)"
            >
              <!-- Content -->
              <IconInline v-if="item.icon" v-bind="item.icon" class="mr-1.5 text-gray-700" />
              <span v-else class="mr-1.5 w-[18px] text-gray-700" />
              <span class="select-none truncate" v-html="item.titleMarked ?? item.title" />
              <span v-if="'path' in item" class="ml-1.5 text-gray-500">
                <span v-html="item.pathMarked ?? item.path" />
              </span>
              <!-- Checked -->
              <i v-if="item.id === modelValue?.id" class="fas fa-check ml-auto pl-4 pr-1 text-gray-700" />
            </li>
          </template>
        </ul>
      </Scroll>
    </div>
  </div>
</template>
