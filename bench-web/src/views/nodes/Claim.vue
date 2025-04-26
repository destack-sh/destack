<script lang="ts" setup>
import { supergraph } from "@/globals";
import { getEnumOptions } from "@/language/core/enum";
import { ClaimData, ClaimTypeOptionInfo, ColorShade, ColorType, EnumType, NodeType, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import Inaccessible from "@/views/builtin/Inaccessible.vue";
import NodeReference from "@/views/builtin/NodeReference.vue";
import Popover from "@/views/builtin/Popover.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, Ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isMinimal">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const nodePtr = toRef(props, "nodePtr");
const { graph, connection } = props.preparedConnection ?? useAutoConnection(nodePtr);
const claim = graph.getRef(nodePtr) as Ref<ClaimData | null>;
const targetPtr = computed(() => claim.value?.targetPtr ?? claim.value?.targetTemplatePtr);
const target = supergraph.getRef(targetPtr.value);

// view
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => nodePtr.value != null && canvas.isSelected(nodePtr.value));

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div
    v-if="claim"
    ref="claimRef"
    class="group/claim flex items-center gap-x-1.5 rounded-sm transition-colors duration-150"
    :class="[
      !isMinimal ? 'border px-1 py-1' : '',
      isSelected ? 'border-gray-400 bg-amber-200' : '',
      !isSelected && (isInspected || isHighlighted) ? 'border-gray-400 bg-gray-100' : '',
      !(isSelected || isInspected || isHighlighted) ? 'border-gray-200 bg-white' : '',
    ]"
    :style="{}"
  >
    <!-- Icon (as big square if not minimal) -->
    <div v-if="!isMinimal" class="flex h-10 w-10 shrink-0 items-center justify-center rounded-sm bg-amber-400">
      <IconInline v-bind="getNodeIcon(target ?? claim)" class="rounded-sm text-center text-lg text-gray-800" />
    </div>

    <!-- TODO :Incomplete: show/control? actual Claim status somehow (what about Entitlements?) -->
    <!-- Target -->
    <NodeReference :node-ptr="targetPtr" :tx="() => connection.tx" is-light :hide-icon="!isMinimal" size="sm" />

    <!-- Meta -->
    <div class="ml-auto">
      <!-- Claim.type -->
      <Popover placement="bottom" :container-margin="8" :reference-margin="4">
        <!-- Button -->
        <template #trigger="{ isOpen, toggle }">
          <!-- Visible -->
          <button
            class="mr-1.5 cursor-pointer rounded-full px-1 transition-colors duration-75 hover:bg-gray-200"
            :class="
              claim.isHidden ? 'text-gray-700 opacity-100' : 'text-gray-400 opacity-0 group-hover/claim:opacity-100'
            "
            @click="connection.tx.update(claim!, { isHidden: !claim.isHidden })"
          >
            <span class="" :class="claim.isHidden ? 'fas fa-eye-slash' : 'fas fa-eye'" />
          </button>
          <!-- Type -->
          <button
            class="cursor-pointer rounded-sm border px-1 text-gray-900 transition-colors duration-75 hover:bg-gray-200"
            :style="{
              backgroundColor: getColorHex(ClaimTypeOptionInfo[claim.type]?.color ?? ColorType.GRAY, ColorShade.S200),
              borderColor: getColorHex(ClaimTypeOptionInfo[claim.type]?.color ?? ColorType.GRAY, ColorShade.S300),
            }"
            @click="toggle"
          >
            <span>{{ ClaimTypeOptionInfo[claim.type]?.text ?? "???" }}</span>
            <span
              class="fas fa-caret-down ml-1.5 text-gray-500 transition-transform duration-75"
              :class="{ 'rotate-180': isOpen }"
            />
          </button>
        </template>
        <!-- Options -->
        <template #content="{ close }">
          <ul class="w-[140px] rounded-sm border border-gray-200 bg-white px-2 py-1.5 shadow-sm shadow-gray-300">
            <li
              v-for="option in getEnumOptions(EnumType.CLAIM_TYPE)"
              :key="option.value"
              role="button"
              class="flex h-[30px] flex-row items-center justify-start rounded-sm px-1.5 transition-colors hover:bg-gray-100"
              @click="() => (connection.tx.update(claim!, { type: option.value }), close())"
            >
              <IconInline v-bind="option.icon" class="mr-1.5 w-5 text-center text-gray-900" />
              <span>{{ option.text }}</span>
            </li>
          </ul>
        </template>
      </Popover>
    </div>
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="connection" />
</template>
