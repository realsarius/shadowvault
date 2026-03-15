import { onMount } from "solid-js";
import { store, refreshLogs } from "../store";
import { LogPanel } from "../components/logs/LogPanel";
import { Button } from "../components/ui/Button";
import { t } from "../i18n";
import styles from "./Logs.module.css";

export function Logs() {
  onMount(() => refreshLogs());

  return (
    <div class={styles.root}>
      <div class={styles.header}>
        <span class={styles.title}>{t("log_title")} ({store.logs.length})</span>
        <Button variant="ghost" size="sm" onClick={() => refreshLogs()}>{t("btn_refresh")}</Button>
      </div>
      <LogPanel logs={store.logs} sources={store.sources} />
    </div>
  );
}
