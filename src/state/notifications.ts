import { useEffect } from "preact/hooks";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { events } from "../lib/bindings";
import { activeId, sessions } from "../state/sessions";

export function useNotifications() {
  useEffect(() => {
    let permitted = false;
    let unlistenStatus: (() => void) | null = null;

    void (async () => {
      permitted = await isPermissionGranted();
      if (!permitted) {
        const result = await requestPermission();
        permitted = result === "granted";
      }

      unlistenStatus = await events.statusChange.listen(async (e) => {
        if (!permitted) return;
        const { sessionId, status } = e.payload;
        if (status !== "needs_input") return;
        if (sessionId === activeId.value) return;
        const session = sessions.value.find((s) => s.id === sessionId);
        if (!session) return;
        sendNotification({
          title: `${session.title} — needs input`,
          body: "Click to focus this session.",
        });
        try {
          await getCurrentWindow().requestUserAttention(1);
        } catch {
          /* not all platforms support this; ignore */
        }
      });
    })();

    return () => {
      unlistenStatus?.();
    };
  }, []);
}
