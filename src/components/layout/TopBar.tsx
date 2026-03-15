import { store } from "../../store";
import { api } from "../../api/tauri";
import { Button } from "../ui/Button";
import styles from "./TopBar.module.css";

const titles: Record<string, string> = {
  dashboard: "Genel Bakış",
  sources:   "Kaynaklar",
  logs:      "Loglar",
  settings:  "Ayarlar",
};

export function TopBar() {
  return (
    <header class={styles.header}>
      <div class={styles.left}>
        <span class={styles.brand}>ShadowVault</span>
        <span class={styles.sep}>|</span>
        <span class={styles.pageTitle}>{titles[store.activePage]}</span>
      </div>
      <div class={styles.right}>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => store.isSchedulerPaused ? api.jobs.resumeAll() : api.jobs.pauseAll()}
        >
          {store.isSchedulerPaused ? "▶ Devam" : "⏸ Duraklat"}
        </Button>
      </div>
    </header>
  );
}
