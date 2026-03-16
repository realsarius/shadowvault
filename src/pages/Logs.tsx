import { createSignal, onMount, Show } from "solid-js";
import { store, refreshLogs, loadMoreLogs } from "../store";
import { LogPanel } from "../components/logs/LogPanel";
import { Button } from "../components/ui/Button";
import { t } from "../i18n";
import styles from "./Logs.module.css";

export function Logs() {
  const [loadingMore, setLoadingMore] = createSignal(false);

  onMount(() => refreshLogs());

  const hasMore = () => store.logs.length < store.logTotal;

  const handleLoadMore = async () => {
    setLoadingMore(true);
    await loadMoreLogs();
    setLoadingMore(false);
  };

  return (
    <div class={styles.root}>
      <div class={styles.header}>
        <span class={styles.title}>
          {t("log_title")} ({store.logs.length}{hasMore() ? `/${store.logTotal}` : ""})
        </span>
        <Button variant="ghost" size="sm" onClick={() => refreshLogs()}>{t("btn_refresh")}</Button>
      </div>
      <LogPanel logs={store.logs} sources={store.sources} />
      <Show when={hasMore()}>
        <div class={styles.loadMoreRow}>
          <Button variant="ghost" size="sm" onClick={handleLoadMore} disabled={loadingMore()}>
            {loadingMore() ? t("log_loading_more") : t("log_load_more").replace("{n}", String(store.logTotal - store.logs.length))}
          </Button>
        </div>
      </Show>
    </div>
  );
}
