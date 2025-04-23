<script lang="ts" setup>
import { autoloader, canvas, supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { NAME_TYPE, TITLE_TYPE } from "@/language/core/type";
import { Transaction } from "@/language/core/transaction";
import {
  AnyNodeData,
  NodeReferenceData,
  NodeType,
  NodeTypeOptionInfo,
  Orientation,
  PROPERTY_ENUM_BY_TYPE,
  TextLineData,
  TextLineType,
} from "@/proto/wire";
import { ConnectionBase } from "@/system/connection";
import { getNodeIcon, IconInline, makeIcon } from "@/ui/icon";
import { PopoverInfoIn } from "@/ui/popover";
import { NODE_REF_CONTEXT_KEY } from "@/ui/space";
import { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import { IS_DEVELOPER_MODE } from "@/utils/globals";
import NodeMetadata from "@/views/builtin/NodeMetadata.vue";
import TextLine from "@/views/content/TextLine.vue";
import { FocusAnchor, NavigationDirection, ViewEmits } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { MaybeElement } from "@vueuse/core";
import { computed, inject, provide, Ref, ref, toRef } from "vue";

const props = defineProps<{
  size: "sm" | "base" | "title" | "inherit";
  node?: AnyNodeData;
  nodePtr?: NodeReferenceData;
  tx?: () => Transaction;
  isUnderline?: boolean;
  isInput?: boolean;
  isLight?: boolean;
  isIconLight?: boolean;
  isMinimal?: boolean;
  isNested?: boolean;
  hideIcon?: boolean;
  hideMetadata?: boolean;
  orientation?: Orientation;
  maxWidth?: number;
}>();
const emit = defineEmits<ViewEmits>();

const iconRef = ref<InstanceType<typeof Icon> | null>(null);
const identifierRef = ref<InstanceType<typeof NativeInput | typeof TextLine> | null>(null);

let node: Ref<AnyNodeData | null | undefined>;
let connection: Ref<ConnectionBase<any, any> | null | undefined> | null;
if (props.node == null) {
  ({ node, connection } = supergraph.getLinkRef(toRef(props, "nodePtr")));
} else {
  node = toRef(props, "node");
  connection = null;
}

const nodeType = computed(() => props.nodePtr?.nodeType ?? (props.node?.metatype as unknown as NodeType));
const nodeTypeName = computed(() => toCamelName(NodeType, nodeType.value));
const nodeProperties = computed(() => (nodeType.value != null ? PROPERTY_ENUM_BY_TYPE[nodeType.value] : null));
const identifierKind = computed(() => {
  if (nodeProperties.value == null) return null;
  else if ("name" in nodeProperties.value) return "name";
  else if ("title" in nodeProperties.value) return "title";
  else return null;
});

// :NodeReferenceStyle
const iconClass = computed(() => [
  props.size == "sm" ? "text-sm w-5 mr-1" : "",
  props.size == "base" ? "text-base w-5 mr-2" : "",
  props.size == "title" ? "text-5xl w-fit mr-3.5" : "",
  props.size == "inherit" ? "max-w-[1em] mr-[0.3em]" : "",
]);
const identifierClass = computed(() => [
  props.size == "sm" ? ["text-sm", props.isLight ? "" : "font-medium"] : "",
  props.size == "base" ? ["text-base", props.isLight ? "" : "font-medium"] : "",
  props.size == "title" ? ["text-4xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.isUnderline ? "underline decoration-gray-300 underline-offset-3" : "",
]);
const metadataClass = computed(() => [
  props.size == "sm" ? "ml-0.5" : "",
  props.size == "base" ? "ml-1" : "",
  props.size == "title" ? "ml-1.5" : "",
  props.size == "inherit" ? "ml-[0.3em]" : "",
]);
const identifierWidthMax = computed(() => {
  if (props.maxWidth != null) return props.maxWidth;
  else if (props.size == "title") return 600;
  else if (props.size == "base") return 400;
  else return 300;
});
const verticalClass = computed(() => [
  props.size == "sm" ? "gap-y-0.5" : "",
  props.size == "base" ? "gap-y-1" : "",
  props.size == "title" ? "gap-y-3" : "",
]);

function getTx() {
  if (connection?.value) {
    return connection.value.tx;
  } else if (props.tx != null) {
    return props.tx();
  } else {
    throw new Error("no transaction provided");
  }
}

// nesting
const isNested = props.isNested ?? inject(NODE_REF_CONTEXT_KEY, null) != null;
provide(NODE_REF_CONTEXT_KEY, node);

defineExpose({
  focusIcon: () => focusInElement(iconRef.value as MaybeElement),
  focusIdentifier: (anchor?: FocusAnchor) => identifierRef.value?.focus?.(anchor),
  focus: (anchor?: FocusAnchor) => {
    identifierRef.value?.focus?.(anchor);
  },
});
</script>
<template>
  <div
    v-if="node != null"
    class=""
    :class="[orientation == Orientation.VERTICAL ? ['flex flex-col', verticalClass] : ['flex flex-row items-baseline']]"
    :data-node-id="node.id"
    :data-node-ck="(node as any).ck"
    :data-node-type="node.metatype"
    :data-node-bench-id="(node as any).benchPtr?.id"
    aria-role="button"
    data-contextmenu-items="space.navigate.open"
    data-suppress-node="self"
    @mousedown.prevent="() => identifierRef?.focus?.('right')"
    @click="
      (e) => {
        if (!isInput && node != null && e.altKey) {
          canvas.goToNode(node);
          e.stopPropagation();
        }
      }
    "
  >
    <IconInline
      v-if="!hideIcon"
      ref="iconRef"
      v-tooltip="{ small: true, text: `Change icon`, isEnabled: isInput } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: (node as any)!.icon, isInput: true },
          isEnabled: isInput,
          onApply: (newIcon) => getTx().update(node!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(node)"
      class="rounded-sm text-center data-[popover=true]:bg-gray-100"
      :class="[
        ...iconClass,
        isInput ? 'hover:cursor-pointer hover:bg-gray-100' : '',
        isIconLight ? '' : 'text-gray-700',
      ]"
      role="button"
      aria-hidden
    />
    <!-- Identifier -->
    <div class="flex max-w-full flex-row items-baseline">
      <NativeInput
        v-if="!isNested && isInput && identifierKind == 'name'"
        id="identifier"
        ref="identifierRef"
        class="shrink-0 rounded-sm text-gray-900"
        :style="{ maxWidth: `${identifierWidthMax}px` }"
        :class="identifierClass"
        :placeholder="nodeTypeName"
        :is-input="!isNested && isInput"
        is-minimal
        :value-type="NAME_TYPE"
        :model-value="(node as any).name"
        @update:model-value="(newValue) => getTx().update(node!, { name: newValue as string }, { debounce: 'long' })"
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
        @mousedown.stop="true /* keep inner focus */"
      />
      <TextLine
        v-else-if="identifierKind == 'title'"
        id="identifier"
        ref="identifierRef"
        truncate
        :model-value="(node as any).title"
        :force-line-type="
          size == 'inherit' ? 'inherit' : size == 'title' ? TextLineType.HEADING_1 : TextLineType.PARAGRAPH
        "
        :hide-mentions="isNested"
        :is-small="size == 'sm'"
        :is-input="!isNested && isInput"
        :placeholder="nodeTypeName"
        :style="{ maxWidth: `${identifierWidthMax}px` }"
        @update:model-value="
          (newValue) => getTx().update(node!, { title: newValue as TextLineData }, { debounce: 'long' })
        "
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
        @keydown.enter.stop="emit('navigate', 'enter')"
        @mousedown.stop="true /* keep inner focus */"
      />
      <span
        v-else
        class="truncate underline decoration-transparent underline-offset-3 transition-colors duration-150 hover:decoration-gray-300"
        :class="[identifierClass]"
        :style="{ maxWidth: `${identifierWidthMax}px` }"
      >
        {{ (node as any).name ?? nodeTypeName }}
      </span>
      <!-- Metadata -->
      <NodeMetadata
        v-if="!isMinimal && !hideMetadata"
        class="ml-1 shrink-0"
        :size="size"
        :node="node"
        :is-light="isLight"
        :class="metadataClass"
      />
    </div>
  </div>
  <div
    v-else
    :class="[orientation == Orientation.VERTICAL ? ['flex flex-col', verticalClass] : ['flex flex-row items-center']]"
  >
    <!-- Node loading -->
    <template v-if="nodePtr != null && autoloader.isPending(nodePtr)">
      <IconInline
        class="w-5 text-center text-gray-400"
        v-bind="makeIcon(NodeTypeOptionInfo[nodePtr?.nodeType!]?.icon ?? 'fas fa-exclamation-triangle')"
      />
      <div class="ml-1 h-2 w-20 rounded-sm bg-gray-100" />
    </template>
    <!-- Node not found (show id in dev mode) -->
    <template v-else-if="IS_DEVELOPER_MODE">
      <span class="text-gray-400">{{ toCamelName(NodeType, nodePtr?.nodeType) }} [{{ nodePtr?.id }}]</span>
    </template>
    <!-- Node not found (show not found) -->
    <template v-else>
      <IconInline
        class="w-5 text-center text-gray-400"
        v-bind="makeIcon(NodeTypeOptionInfo[nodePtr?.nodeType!]?.icon ?? 'fas fa-exclamation-triangle')"
      />
      <span class="ml-1 text-gray-400">[{{ nodePtr?.id != null ? "missing" : "empty" }}]</span>
    </template>
  </div>
</template>
