<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { renderText } from "@/language/core/text";
import { NAME_TYPE } from "@/language/core/type";
import { isRunActive } from "@/language/runtime/run";
import {
  ActionType,
  ActionTypeOptionInfo,
  ColorShade,
  FieldType,
  NodeType,
  Orientation,
  PortSide,
  ViewData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/runtime/runtime";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { type ActionMapKit } from "@/ui/action";
import { getActionSides, useFlowContextMaybe } from "@/ui/flow";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { PopoverInfoIn, pushDefaultMenu } from "@/ui/popover";
import { getNodeColorHex, getRunColorHex } from "@/ui/style";
import { focusInElement } from "@/ui/view";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import RunStatus from "@/views/builtins/RunStatus.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { MaybeElement } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const actionPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.ACTION>);
const { connection, graph } = props.preparedConnection ?? useExistingConnection(actionPtr);
const flowCtx = useFlowContextMaybe();
const actionState = flowCtx?.actionsStates.value[actionPtr.value.id!]; // must exist
const action = actionState?.action ?? graph.getRef(actionPtr.value);
const toolPtr = actionState?.toolPtr ?? computed(() => action.value?.toolPtr);
const tool = actionState?.tool ?? graph.getRef(toolPtr.value);
const fields = actionState?.fields ?? graph.getChildrenRef(actionPtr.value, NodeType.FIELD);
const isInspected = computed(() => canvas.isInspected(actionPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(actionPtr.value));
const isSelected = computed(() => state.isSelected(actionPtr.value));
const sides = computed(() => (action.value != null ? getActionSides(action.value) : []));

const nameRef: Ref<InstanceType<typeof NativeInput> | null> = ref(null);
const containerRef: Ref<HTMLElement | null> = ref(null);

// run
const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ id: actionPtr.value?.id }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ id: actionPtr.value?.id }));

//
// Interaction
//

// actions
const actions: Partial<ActionMapKit<"space">> & ActionMapKit<"action"> = {
  // space
  "space.edit.rename": {
    action: () => {
      nextTick(() => focusInElement(nameRef.value as MaybeElement));
    },
  },
};

defineExpose<ViewExpose>({ self, id, actions });
</script>
<template>
  <div
    v-if="action"
    ref="containerRef"
    class="group/action flex flex-row items-center gap-x-2.5 rounded border px-1 py-1 outline outline-2 transition-colors duration-150"
    :class="[
      flowCtx != null ? '' : 'relative',
      isSelected ? 'border-gray-400 bg-orange-100' : '',
      !isSelected && (isInspected || isHighlighted) ? 'border-gray-400 bg-gray-100' : '',
      !(isSelected || isInspected || isHighlighted) ? 'border-gray-200 bg-white' : '',
      lastRun != null && isRunActive(lastRun) ? '' : 'outline-transparent',
    ]"
    :style="{
      borderColor: lastRun != null ? getRunColorHex(lastRun.status) : '',
      outlineColor: lastRun != null && isRunActive(lastRun) ? getRunColorHex(lastRun.status) : '',
    }"
    aria-role="button"
    @mouseup="(e) => flowCtx?.endDragging(e, { kind: 'action', action: action! })"
    @mousedown.alt="
      (e) => {
        flowCtx?.startDragging(e, { kind: 'port', action: action!, side: PortSide.OUTGOING });
      }
    "
  >
    <!-- Icon -->
    <div
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: action?.icon, isInput: true },
          isEnabled: true,
          onApply: (newIcon) => connection.tx.update(action!, { icon: newIcon }),
        })
      "
      v-tooltip="{ small: true, text: `Change icon` }"
      class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded  hover:cursor-pointer hover:saturate-200"
      :style="{
        backgroundColor: getNodeColorHex(action, ColorShade.S500),
      }"
    >
      <IconInline
        ref="iconRef"
        v-bind="getNodeIcon(action, { base: tool })"
        class="rounded text-center text-lg text-gray-800"
      />
    </div>

    <!-- Main -->
    <div
      class="flex flex-1 flex-col gap-y-0.5"
      :style="{
        maxWidth: `calc(100% - 60px)`,
      }"
    >
      <!-- Header -->
      <div class="flex h-5 flex-row gap-x-1.5">
        <!-- Name -->
        <NativeInput
          id="name"
          ref="nameRef"
          class="flex-shrink-0 font-medium transition-colors duration-150"
          :placeholder="toCamelName(ActionType, action.type)"
          is-input
          is-minimal
          :value-type="NAME_TYPE"
          :model-value="action.name"
          @update:model-value="
            (newValue) => connection.tx.update(action!, { name: newValue as string }, { debounce: 'long' })
          "
        />
        <!-- Link (if tool) -->
        <button
          v-if="tool"
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-gray-700"
          @click.stop="canvas.goToNode(tool)"
        >
          <i class="fas fa-arrow-up-right" />
        </button>
        <!-- Metadata -->
        <NodeMetadata :node="action" size="sm" />
        <!-- Controls/Meta -->
        <div class="ml-auto flex flex-row pl-2 pr-1.5">
          <!-- Run status -->
          <button v-if="lastRun != null" class="rounded hover:bg-gray-100" aria-hidden>
            <RunStatus :run="lastRun" :orientation="Orientation.HORIZONTAL_REVERSED" icon="dot" />
          </button>
        </div>
      </div>
      <!-- Content -->
      <div class="flex h-5 max-w-full flex-row items-center gap-x-1 truncate text-gray-700">
        <!-- NOTE :Incomplete: better Action body -->
        <!-- Fields -->
        <span
          v-for="field in fields.filter((f) => f.type == FieldType.INPUT)"
          :key="field.id"
          class="truncate transition-colors duration-75"
          :class="canvas.isHighlighted(field) ? 'text-gray-700' : 'text-gray-400'"
        >
          {{ field.name }}
        </span>
        <i v-if="fields.length != 0" class="fas fa-arrow-right text-xs text-gray-400" />
        <span
          v-for="field in fields.filter((f) => f.type == FieldType.OUTPUT)"
          :key="field.id"
          class="truncate transition-colors duration-75"
          :class="canvas.isHighlighted(field) ? 'text-gray-700' : 'text-gray-400'"
        >
          {{ field.name }}
        </span>
        <span v-if="fields.length == 0" class="truncate text-gray-400">
          {{ (action.text != null ? renderText(action.text) : null) ?? ActionTypeOptionInfo[action.type]?.text }}
        </span>
      </div>
      <!-- Tools/Tags -->
      <!-- TODO :UX: Action tools/tags? -->
    </div>

    <!-- Floating Menu -->
    <div
      class="absolute flex flex-row gap-x-1.5"
      :class="[flowCtx != null ? '-left-5 top-0 -translate-x-1' : 'right-0 top-0']"
    >
      <button
        aria-hidden
        class="text-gray-400 opacity-0 transition-colors duration-75 hover:text-gray-700 group-hover/action:opacity-100 data-[popover=true]:text-gray-700 data-[popover=true]:opacity-100"
        @click="(e) => pushDefaultMenu('main', action!, e)"
      >
        <i class="fas fa-ellipsis-vertical w-5 text-center" />
      </button>
    </div>

    <!-- NOTE :UX: show last action output/error here? -->
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- Should never be rendered by containing flow -->
  </div>
</template>
