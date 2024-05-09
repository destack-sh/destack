<script lang="ts" setup>
import { ViewData, NodeType, Variant, ColorShade, FieldZone } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { IconInline, getNodeIcon } from "@/system/icon";
import type { ActionMapImplementation } from "@/system/action";
import Icon from "@/views/content/Icon.vue";
import { type PopoverInfo, type PopoverInfoIn } from "@/utils/menu";
import type { TooltipInfo } from "@/utils/tooltip";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<HTMLElement | null>(null);

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.FIELD>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `field.${nodePtr.value.id}` },
    computed(() => ({ roots: [nodePtr.value], isEnabled: nodePtr.value != null })),
  );
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex w-fit flex-row items-center rounded border bg-gray-50 px-1.5 py-[3px]"
    :class="[
      inspectionPtr?.id == field.id
        ? 'border-primary-900'
        : [variant != Variant.STEALTH ? 'border-gray-200' : 'border-gray-200', 'hover:border-gray-300'],
    ]"
  >
    <!-- TODO :UX!: Field is annoying (should be double-click to edit, change type in contextmenu, etc.) -->
    <IconInline
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: field!.icon },
          onApply: (newIcon) => pkgConnection.tx.update(field!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(field)"
      class="mr-0.5 w-6 rounded border border-transparent p-0.5 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:border-primary-900 data-[popover=true]:bg-gray-100"
    />
    <input
      ref="nameRef"
      class="w-fit min-w-fit max-w-fit truncate rounded border-0 bg-transparent font-medium outline-none ring-0 hover:bg-gray-100 focus:ring-0"
      spellcheck="false"
      :value="field.name"
      :size="Math.max(field.name?.length ?? 0, 5)"
      @input="pkgConnection.tx.update(field!, { name: ($event.target as HTMLInputElement).value })"
    />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
