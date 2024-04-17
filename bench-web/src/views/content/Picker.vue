<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType, BenchType } from "@/proto/wire";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { ref, toRef, type Ref } from "vue";
import { makeViewId } from "@/views";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { getEnumOptions, toCamelName, type EnumOption } from "@/system/lang";
import { EnumType } from "@/proto/wire";
import type { EnumOptionItem } from "@/system/search";
import type { NodeItem } from "@/system/search";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Pick<
    ViewData,
    "title" | "text" | "icon" | "valuePacked" | "valueType" | "isInput" | "isInline" | "isDisabled"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);

const options = getEnumOptions(EnumType.BLOCK_TYPE); // nocheckin: get generic options (use SearchIndex)
const results = options; // nocheckin: search results

//
// Interaction
//

function apply(option: EnumOptionItem | NodeItem) {
  emit("update:modelValue", option);
  emit("apply", option);
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  return true; // nocheckin: interaction
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <!-- nocheckin: Picker variants/isInput/isInline/isDisabled/... -->
  <div>
    Pick:{{ toCamelName(BenchType, valueType?.benchType) }}
    <div>
      <!-- Query -->
      <input ref="queryRef" type="text" class="bg-transparent" />
      <!-- Results -->
      <ul class="flex flex-col">
        <li v-for="option in results" :key="option.id" @click="apply(option)">
          {{ option.title }}
        </li>
      </ul>
    </div>
    <!-- nocheckin -->
  </div>
</template>
