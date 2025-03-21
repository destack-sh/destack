import { ToastLevel } from "@/ui/toast";
import { getAllTransactionBuffers } from "@/language/runtime/transaction";
import { ViewType } from "@/proto/wire";
import { isDeveloperMode } from "@/system/client";
import { provideCommands } from "@/ui/command";
import { Casing } from "@/utils/string";
import { makeIcon } from "@/ui/icon";
import { toaster } from "@/ui/toast";
import { generateRandomName } from "@/utils/naming";
import { toCasing } from "@/utils/string";
import { canvas } from "@/globals";

// debug commands
export const DEBUG_COMMANDS = provideCommands<"debug">({
  // test
  "developer.test.developerMode": {
    type: "toggle",
    icon: "fas fa-binary",
    title: "Developer Mode",
    text: "Developer Mode enables some advanced and some weird features.",
    isChecked: isDeveloperMode,
    command: () => {
      isDeveloperMode.value = !isDeveloperMode.value;
      toaster.success({
        override: "developer.toggleDeveloperMode",
        title: isDeveloperMode.value ? "Developer Mode Enabled" : "Developer Mode Disabled",
        text: isDeveloperMode.value ? "Welcome to the dark side." : "Back to the normal side.",
        icon: "fas fa-binary",
        commands: [
          {
            title: isDeveloperMode.value ? "Disable" : "Enable",
            icon: makeIcon({ faName: isDeveloperMode.value ? "fas fa-toggle-off" : "fas fa-toggle-on" }),
            command: () => {
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
    command: () => {
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
    command: () => {
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
    command: () => testToast(ToastLevel.INFO),
  },
  "developer.toast.debug": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Debug Toast",
    text: "Show a debug toast",
    command: () => testToast(ToastLevel.DEBUG),
  },
  "developer.toast.error": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-exclamation-triangle",
    title: "Error Toast",
    text: "Show an error toast",
    command: () => testToast(ToastLevel.ERROR),
  },
  "developer.toast.success": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-check-circle",
    title: "Success Toast",
    text: "Show a success toast",
    command: () => testToast(ToastLevel.SUCCESS),
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
