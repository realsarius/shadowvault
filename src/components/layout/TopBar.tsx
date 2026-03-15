import { store } from "../../store";
import { api } from "../../api/tauri";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import styles from "./TopBar.module.css";

const pageTitleKeys = {
  dashboard: "nav_dashboard",
  sources:   "nav_sources",
  logs:      "nav_logs",
  settings:  "nav_settings",
} as const;

export function TopBar() {
  return (
    <header class={styles.header}>
      <div class={styles.left}>
        <span class={styles.brand}>{t("topbar_brand")}</span>
        <span class={styles.sep}>|</span>
        <span class={styles.pageTitle}>{t(pageTitleKeys[store.activePage])}</span>
      </div>
      <div class={styles.right}>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => store.isSchedulerPaused ? api.jobs.resumeAll() : api.jobs.pauseAll()}
        >
          {store.isSchedulerPaused ? t("topbar_resume") : t("topbar_pause")}
        </Button>
      </div>
    </header>
  );
}
