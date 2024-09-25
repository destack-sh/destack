<script lang="ts" setup>
import { NAME_CONSTRAINT } from "@/language/const";
import { pathToSvg, PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { isGeneratedNodeName } from "@/language/node";
import { ColorShade, ColorType, NodeType, PipeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, inspectionPtr } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { getColorHex } from "@/ui/style";
import { getNativeConstraintProps, guardNativeNameInput } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const pipePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.PIPE>);
const flowCtx = useFlowContext();
const state = flowCtx.pipesStates.value[pipePtr.value.id!]; // must exist
const { pipe, source, target, path, sourcePort, targetPort } = state;
const pathSvg = computed(() => (path.value != null ? pathToSvg(path.value.points) : undefined));
const pathColorHex = computed(() => getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S600));

const isHidden = computed(() => pipe.value?.isHidden && !isInspected.value);
const isGeneratedName = computed(() => pipe.value != null && isGeneratedNodeName(pipe.value.metatype, pipe.value.name));
//
// Interaction
//

const isInspected = computed(() => inspectionPtr.value?.id == pipePtr.value?.id);

// actions
const actions: Partial<ActionMapImplementation<"common" | "pipe">> = {
  "pipe.edit.isControl": {
    action: () => {
      if (pipe.value == null) return false;
      const newType = pipe.value.type == PipeType.CONTROL_AND_DATA ? PipeType.DATA : PipeType.CONTROL_AND_DATA;
      flowCtx.tx.update(pipe.value, { type: newType }, { debounce: "tick" });
    },
  },
  "pipe.edit.isHidden": {
    action: () => {
      if (pipe.value == null) return false;
      flowCtx.tx.update(pipe.value, { isHidden: !pipe.value.isHidden }, { debounce: "tick" });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div v-if="pipe != null && path != null" :class="isHidden ? 'group pointer-events-none z-30' : ''">
    <!-- Background/outline path for highlighting (and larger hit area) -->
    <svg
      class="absolute cursor-pointer overflow-visible blur-sm transition-colors duration-150"
      :class="isInspected ? 'opacity-80' : pipe.isHidden ? 'opacity-0' : 'opacity-0 hover:opacity-50'"
      :style="{ color: pathColorHex }"
    >
      <path
        :stroke-width="PIPE_WIDTH + 2"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :d="pathSvg"
      />
    </svg>
    <!-- Primary path -->
    <svg
      class="pointer-events-none absolute overflow-visible transition-colors duration-150"
      :class="pipe.isHidden ? (isInspected ? 'opacity-70' : 'opacity-0') : 'opacity-100'"
      :style="{ color: pathColorHex }"
    >
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :stroke-dasharray="pipe.type === PipeType.DATA ? `${1},${PIPE_WIDTH * 2}` : undefined"
        :d="pathSvg"
      />
    </svg>
    <!-- Midpoint stuff -->
    <div
      v-if="!isHidden"
      class="absolute select-none transition-colors duration-150"
      :class="[
        path.isMidpointHorizontal ? '-translate-x-1/2 -translate-y-1/2 pb-5' : '-translate-x-1/2 -translate-y-1/2',
        isGeneratedName && !isInspected ? 'opacity-0' : 'opacity-100',
        isInspected || !isGeneratedName ? 'text-gray-700' : 'text-gray-400',
      ]"
      :style="{ left: path.midpoint.x + 'px', top: path.midpoint.y + 'px' }"
    >
      <input
        ref="nameRef"
        type="text"
        class="w-fit min-w-fit max-w-fit rounded border-0 bg-transparent text-xs font-medium outline-none ring-0 focus:ring-0"
        spellcheck="false"
        data-suppress-drag="true"
        :value="pipe.name"
        :size="Math.max(pipe.name.length, 3)"
        v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
        @input="
          guardNativeNameInput($event, pipe!.name, (newValue) =>
            flowCtx.tx.update(pipe!, { name: newValue }, { debounce: 'long' }),
          )
        "
      />
    </div>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
