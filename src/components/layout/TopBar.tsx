import { TbOutlineSun, TbOutlineMoon, TbOutlinePlayerPause, TbOutlinePlayerPlay } from "solid-icons/tb";
import { store, loadSettings } from "../../store";
import { api } from "../../api/tauri";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import styles from "./TopBar.module.css";

import type { TKey } from "../../i18n";

const pageTitleKeys: Record<string, TKey> = {
  dashboard: "nav_dashboard",
  sources:   "nav_sources",
  logs:      "nav_logs",
  settings:  "nav_settings",
  license:   "nav_license",
};

export function TopBar() {
  const currentTheme = () => store.settings?.theme ?? "dark";
  const pageTitle = () => {
    const key = pageTitleKeys[store.activePage];
    return key ? t(key) : store.activePage;
  };

  const toggleTheme = async () => {
    const next = currentTheme() === "dark" ? "light" : "dark";
    await api.settings.update({ ...store.settings!, theme: next });
    await loadSettings();
  };

  return (
    <header class={styles.header}>
      <div class={styles.left}>
        <span class={styles.brand}>{t("topbar_brand")}</span>
        <span class={styles.sep}>|</span>
        <span class={styles.pageTitle}>{pageTitle()}</span>
      </div>
      <div class={styles.right}>
        <button class={styles.themeBtn} onClick={toggleTheme} title="Temayı değiştir">
          {currentTheme() === "dark"
            ? <TbOutlineSun size={16} />
            : <TbOutlineMoon size={16} />}
        </button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => store.isSchedulerPaused ? api.jobs.resumeAll() : api.jobs.pauseAll()}
        >
          {store.isSchedulerPaused
            ? <><TbOutlinePlayerPlay size={14} />{" "}{t("topbar_resume")}</>
            : <><TbOutlinePlayerPause size={14} />{" "}{t("topbar_pause")}</>}
        </Button>
      </div>
    </header>
  );
}
