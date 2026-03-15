import { onMount } from "solid-js";
import { store, refreshLogs } from "../store";
import { LogPanel } from "../components/logs/LogPanel";
import { Button } from "../components/ui/Button";
import styles from "./Logs.module.css";

export function Logs() {
  onMount(() => refreshLogs());

  return (
    <div class={styles.root}>
      <div class={styles.header}>
        <span class={styles.title}>Log Kayıtları ({store.logs.length})</span>
        <Button variant="ghost" size="sm" onClick={() => refreshLogs()}>↻ Yenile</Button>
      </div>
      <LogPanel logs={store.logs} sources={store.sources} />
    </div>
  );
}
