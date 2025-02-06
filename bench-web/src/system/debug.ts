import { ToastLevel } from "@/ui/toast";
import { getAllTransactionBuffers } from "@/language/runtime/transaction";
import { ViewType } from "@/proto/wire";
import { isDeveloperMode } from "@/system/client";
import { provideActions } from "@/ui/action";
import { Casing } from "@/utils/string";
import { makeIcon } from "@/ui/icon";
import { toaster } from "@/ui/toast";
import { generateRandomName } from "@/utils/naming";
import { toCasing } from "@/utils/string";
import { canvas } from "@/globals";

// debug actions
export const DEBUG_ACTIONS = provideActions<"debug">({
  // test
  "developer.test.developerMode": {
    type: "toggle",
    icon: "fas fa-binary",
    title: "Developer Mode",
    text: "Developer Mode enables some advanced and some weird features.",
    isChecked: isDeveloperMode,
    action: () => {
      isDeveloperMode.value = !isDeveloperMode.value;
      toaster.success({
        override: "developer.toggleDeveloperMode",
        title: isDeveloperMode.value ? "Developer Mode Enabled" : "Developer Mode Disabled",
        text: isDeveloperMode.value ? "Welcome to the dark side." : "Back to the normal side.",
        icon: "fas fa-binary",
        actions: [
          {
            title: isDeveloperMode.value ? "Disable" : "Enable",
            icon: makeIcon({ faName: isDeveloperMode.value ? "fas fa-toggle-off" : "fas fa-toggle-on" }),
            action: () => {
              isDeveloperMode.value = !isDeveloperMode.value;
            },
          },
        ],
      });
    },
    shortcuts: ["alt+f12", "f12"],
  },
  "developer.test.retryAllFailed": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-redo",
    title: "Retry Commits",
    text: "Retry all current failed transactions",
    action: () => {
      getAllTransactionBuffers().forEach((txBuffer) => {
        Object.values(txBuffer.failedCommits?.value ?? {}).forEach((commit) => txBuffer.retry!(commit.id));
      });
    },
  },
  "developer.test.addEmptyView": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-window-frame",
    title: "Add Empty View",
    text: "Adds an empty debug view to this root",
    action: () => {
      const name = toCasing(generateRandomName().toUpperCase(), Casing.CAMEL, true);
      canvas.addView({ type: ViewType.EMPTY, name, title: name });
    },
  },
  // toast
  "developer.toast.info": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-info-circle",
    title: "Info Toast",
    text: "Show an info toast",
    action: () => testToast(ToastLevel.INFO),
  },
  "developer.toast.debug": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Debug Toast",
    text: "Show a debug toast",
    action: () => testToast(ToastLevel.DEBUG),
  },
  "developer.toast.error": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-exclamation-triangle",
    title: "Error Toast",
    text: "Show an error toast",
    action: () => testToast(ToastLevel.ERROR),
  },
  "developer.toast.success": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-check-circle",
    title: "Success Toast",
    text: "Show a success toast",
    action: () => testToast(ToastLevel.SUCCESS),
  },
});

function testToast(level: ToastLevel) {
  toaster.add({
    level,
    title: toCasing(ToastLevel[level], Casing.CAMEL),
    text: "This is a test toast. Lorem ipsum dolor sit amet. Much more text.",
    durationMs: 60000,
  });
}
