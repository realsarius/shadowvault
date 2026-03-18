import { createMemo, createSignal, For, onMount, Show } from "solid-js";
import { toast } from "solid-sonner";
import { api, type LogExportFormat, type LogQueryFilters } from "../api/tauri";
import { store } from "../store";
import { t, ti } from "../i18n";
import type { LogEntry } from "../store/types";
import { Button } from "../components/ui/Button";
import { LogPanel } from "../components/logs/LogPanel";
import styles from "./Logs.module.css";

const LOG_PAGE_SIZE = 50;
type DatePreset = "all" | "today" | "7d" | "30d" | "custom";
type QuickView = "all" | "errors";

interface LogSummary {
  total: number;
  success: number;
  failed: number;
  running: number;
}

function getDateRange(preset: DatePreset, fromDate: string, toDate: string): { startedAfter?: string; startedBefore?: string } | null {
  const now = new Date();
  if (preset === "all") return {};

  if (preset === "today") {
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);
    return { startedAfter: start.toISOString() };
  }

  if (preset === "7d" || preset === "30d") {
    const days = preset === "7d" ? 7 : 30;
    const start = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);
    return { startedAfter: start.toISOString() };
  }

  if (preset !== "custom") return {};
  if (!fromDate && !toDate) return {};

  const start = fromDate ? new Date(`${fromDate}T00:00:00`) : null;
  const end = toDate ? new Date(`${toDate}T23:59:59.999`) : null;

  if ((start && Number.isNaN(start.getTime())) || (end && Number.isNaN(end.getTime()))) return null;
  if (start && end && start.getTime() > end.getTime()) return null;

  return {
    startedAfter: start?.toISOString(),
    startedBefore: end?.toISOString(),
  };
}

function pickActiveFilters(filters: LogQueryFilters): Omit<LogQueryFilters, "limit" | "offset"> {
  const next: Omit<LogQueryFilters, "limit" | "offset"> = {};
  if (filters.sourceId) next.sourceId = filters.sourceId;
  if (filters.destinationId) next.destinationId = filters.destinationId;
  if (filters.status) next.status = filters.status;
  if (filters.startedAfter) next.startedAfter = filters.startedAfter;
  if (filters.startedBefore) next.startedBefore = filters.startedBefore;
  if (filters.searchText) next.searchText = filters.searchText;
  return next;
}

export function Logs() {
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  const [total, setTotal] = createSignal(0);
  const [summary, setSummary] = createSignal<LogSummary>({ total: 0, success: 0, failed: 0, running: 0 });
  const [loading, setLoading] = createSignal(false);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [busyDelete, setBusyDelete] = createSignal(false);
  const [exportingFormat, setExportingFormat] = createSignal<LogExportFormat | null>(null);

  const [quickView, setQuickView] = createSignal<QuickView>("all");
  const [sourceFilter, setSourceFilter] = createSignal("all");
  const [statusFilter, setStatusFilter] = createSignal("all");
  const [searchText, setSearchText] = createSignal("");
  const [datePreset, setDatePreset] = createSignal<DatePreset>("7d");
  const [fromDate, setFromDate] = createSignal("");
  const [toDate, setToDate] = createSignal("");
  const [clearDays, setClearDays] = createSignal(30);

  const [appliedFilters, setAppliedFilters] = createSignal<Omit<LogQueryFilters, "limit" | "offset">>({});

  const hasMore = createMemo(() => logs().length < total());
  const loadedText = createMemo(() => `${logs().length}/${total()}`);

  const buildFilters = (): Omit<LogQueryFilters, "limit" | "offset"> | null => {
    const dateRange = getDateRange(datePreset(), fromDate(), toDate());
    if (dateRange === null) return null;
    const resolvedStatus = quickView() === "errors"
      ? "Failed"
      : (statusFilter() === "all" ? undefined : statusFilter());

    const raw: LogQueryFilters = {
      sourceId: sourceFilter() === "all" ? undefined : sourceFilter(),
      status: resolvedStatus,
      searchText: searchText().trim() || undefined,
      startedAfter: dateRange.startedAfter,
      startedBefore: dateRange.startedBefore,
    };
    return pickActiveFilters(raw);
  };

  const refreshSummary = async (base: Omit<LogQueryFilters, "limit" | "offset">) => {
    const summaryBase = { ...base };
    delete summaryBase.status;

    const [all, success, failed, running] = await Promise.all([
      api.logs.count(summaryBase),
      api.logs.count({ ...summaryBase, status: "Success" }),
      api.logs.count({ ...summaryBase, status: "Failed" }),
      api.logs.count({ ...summaryBase, status: "Running" }),
    ]);
    setSummary({ total: all, success, failed, running });
  };

  const fetchFirstPage = async () => {
    const base = buildFilters();
    if (base === null) {
      toast.error(t("log_invalid_date_range"));
      return;
    }

    setLoading(true);
    try {
      const [firstPage, count] = await Promise.all([
        api.logs.get({ ...base, limit: LOG_PAGE_SIZE, offset: 0 }),
        api.logs.count(base),
      ]);
      setAppliedFilters(base);
      setLogs(firstPage);
      setTotal(count);
      await refreshSummary(base);
    } catch (e: any) {
      toast.error(ti("log_fetch_error", { err: e?.message ?? String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const resetFilters = async () => {
    setQuickView("all");
    setSourceFilter("all");
    setStatusFilter("all");
    setSearchText("");
    setDatePreset("7d");
    setFromDate("");
    setToDate("");
    await fetchFirstPage();
  };

  const handleQuickViewChange = async (view: QuickView) => {
    if (quickView() === view) return;
    setQuickView(view);
    await fetchFirstPage();
  };

  const handleLoadMore = async () => {
    if (!hasMore()) return;
    setLoadingMore(true);
    try {
      const more = await api.logs.get({
        ...appliedFilters(),
        limit: LOG_PAGE_SIZE,
        offset: logs().length,
      });
      setLogs((prev) => [...prev, ...more]);
    } catch (e: any) {
      toast.error(ti("log_fetch_error", { err: e?.message ?? String(e) }));
    } finally {
      setLoadingMore(false);
    }
  };

  const handleDeleteOne = async (log: LogEntry) => {
    if (busyDelete()) return;
    const ok = confirm(ti("log_delete_one_confirm", { id: log.id }));
    if (!ok) return;

    setBusyDelete(true);
    try {
      const deleted = await api.logs.deleteEntry(log.id);
      if (deleted > 0) {
        setLogs((prev) => prev.filter((x) => x.id !== log.id));
        setTotal((prev) => Math.max(0, prev - deleted));
        await refreshSummary(appliedFilters());
        toast.success(ti("log_deleted_n", { n: deleted }));
      }
    } catch (e: any) {
      toast.error(ti("log_fetch_error", { err: e?.message ?? String(e) }));
    } finally {
      setBusyDelete(false);
    }
  };

  const handleDeleteFiltered = async () => {
    if (busyDelete()) return;
    if (total() === 0) return;

    const ok = confirm(ti("log_delete_filtered_confirm", { n: total() }));
    if (!ok) return;

    setBusyDelete(true);
    try {
      const deleted = await api.logs.clear(appliedFilters());
      toast.success(ti("log_deleted_n", { n: deleted }));
      await fetchFirstPage();
    } catch (e: any) {
      toast.error(ti("log_fetch_error", { err: e?.message ?? String(e) }));
    } finally {
      setBusyDelete(false);
    }
  };

  const handleClearOld = async () => {
    const days = clearDays();
    if (!days || days < 1) return;
    const ok = confirm(ti("log_clear_old_confirm", { n: days }));
    if (!ok) return;

    setBusyDelete(true);
    try {
      const deleted = await api.logs.clearOld(days);
      toast.success(ti("log_deleted_n", { n: deleted }));
      await fetchFirstPage();
    } catch (e: any) {
      toast.error(ti("log_fetch_error", { err: e?.message ?? String(e) }));
    } finally {
      setBusyDelete(false);
    }
  };

  const handleExport = async (format: LogExportFormat) => {
    if (exportingFormat()) return;
    setExportingFormat(format);
    try {
      const path = await api.logs.export(format, appliedFilters());
      toast.success(ti("log_export_success", { path }));
    } catch (e: any) {
      if (e?.message !== "cancelled") {
        toast.error(ti("log_export_error", { err: e?.message ?? String(e) }));
      }
    } finally {
      setExportingFormat(null);
    }
  };

  onMount(() => {
    fetchFirstPage();
  });

  return (
    <div class={styles.root}>
      <div class={styles.header}>
        <div class={styles.titleWrap}>
          <span class={styles.title}>{t("log_title")}</span>
          <span class={styles.loaded}>{t("log_loaded_count").replace("{n}", loadedText())}</span>
        </div>
        <div class={styles.headerActions}>
          <Button variant="ghost" size="sm" onClick={() => handleExport("csv")} disabled={!!exportingFormat()}>
            {exportingFormat() === "csv" ? t("log_exporting") : t("log_export_csv")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => handleExport("json")} disabled={!!exportingFormat()}>
            {exportingFormat() === "json" ? t("log_exporting") : t("log_export_json")}
          </Button>
          <Button variant="ghost" size="sm" onClick={fetchFirstPage} disabled={loading() || loadingMore()}>
            {t("btn_refresh")}
          </Button>
          <Button variant="danger" size="sm" onClick={handleDeleteFiltered} disabled={busyDelete() || total() === 0}>
            {t("log_delete_filtered")}
          </Button>
        </div>
      </div>

      <div class={styles.summaryGrid}>
        <div class={styles.summaryCard}>
          <span class={styles.summaryLabel}>{t("log_summary_total")}</span>
          <strong class={styles.summaryValue}>{summary().total}</strong>
        </div>
        <div class={styles.summaryCard}>
          <span class={styles.summaryLabel}>{t("log_summary_success")}</span>
          <strong class={styles.summaryValue}>{summary().success}</strong>
        </div>
        <div class={styles.summaryCard}>
          <span class={styles.summaryLabel}>{t("log_summary_failed")}</span>
          <strong class={styles.summaryValue}>{summary().failed}</strong>
        </div>
        <div class={styles.summaryCard}>
          <span class={styles.summaryLabel}>{t("log_summary_running")}</span>
          <strong class={styles.summaryValue}>{summary().running}</strong>
        </div>
      </div>

      <div class={styles.quickTabs}>
        <button
          class={styles.quickTab}
          data-active={quickView() === "all"}
          onClick={() => void handleQuickViewChange("all")}
        >
          {t("log_quick_all")}
        </button>
        <button
          class={styles.quickTab}
          data-active={quickView() === "errors"}
          onClick={() => void handleQuickViewChange("errors")}
        >
          {t("log_quick_errors")}
        </button>
      </div>

      <div class={styles.filterBar}>
        <div class={styles.filterGroup}>
          <label class={styles.label}>{t("log_filter")}</label>
          <select class={styles.select} value={sourceFilter()} onChange={(e) => setSourceFilter(e.currentTarget.value)}>
            <option value="all">{t("log_all_sources")}</option>
            <For each={store.sources}>{(s) => <option value={s.id}>{s.name}</option>}</For>
          </select>
          <select
            class={styles.select}
            value={statusFilter()}
            onChange={(e) => setStatusFilter(e.currentTarget.value)}
            disabled={quickView() === "errors"}
          >
            <option value="all">{t("log_all_statuses")}</option>
            <option value="Success">{t("status_success")}</option>
            <option value="Failed">{t("status_failed")}</option>
            <option value="Running">{t("status_running")}</option>
            <option value="Skipped">{t("status_skipped")}</option>
            <option value="Cancelled">{t("status_cancelled")}</option>
          </select>
        </div>

        <div class={styles.filterGroup}>
          <input
            class={styles.search}
            type="text"
            value={searchText()}
            onInput={(e) => setSearchText(e.currentTarget.value)}
            placeholder={t("log_search_ph")}
          />
          <select class={styles.select} value={datePreset()} onChange={(e) => setDatePreset(e.currentTarget.value as DatePreset)}>
            <option value="all">{t("log_date_all")}</option>
            <option value="today">{t("log_date_today")}</option>
            <option value="7d">{t("log_date_7d")}</option>
            <option value="30d">{t("log_date_30d")}</option>
            <option value="custom">{t("log_date_custom")}</option>
          </select>
          <Show when={datePreset() === "custom"}>
            <input class={styles.dateInput} type="date" value={fromDate()} onInput={(e) => setFromDate(e.currentTarget.value)} />
            <input class={styles.dateInput} type="date" value={toDate()} onInput={(e) => setToDate(e.currentTarget.value)} />
          </Show>
          <Button variant="ghost" size="sm" onClick={fetchFirstPage} disabled={loading()}>
            {t("log_apply_filters")}
          </Button>
          <Button variant="ghost" size="sm" onClick={resetFilters} disabled={loading()}>
            {t("log_reset_filters")}
          </Button>
        </div>
      </div>

      <div class={styles.retentionBar}>
        <span class={styles.label}>{t("log_clear_old_label")}</span>
        <input
          class={styles.daysInput}
          type="number"
          min="1"
          value={clearDays()}
          onInput={(e) => setClearDays(parseInt(e.currentTarget.value, 10) || 30)}
        />
        <span class={styles.daysLabel}>{t("set_days")}</span>
        <Button variant="danger" size="sm" onClick={handleClearOld} disabled={busyDelete()}>
          {t("log_clear_old_btn")}
        </Button>
      </div>

      <Show when={!loading()} fallback={<div class={styles.loading}>{t("log_loading_more")}</div>}>
        <LogPanel logs={logs()} sources={store.sources} onDelete={handleDeleteOne} />
      </Show>

      <Show when={hasMore()}>
        <div class={styles.loadMoreRow}>
          <Button variant="ghost" size="sm" onClick={handleLoadMore} disabled={loadingMore()}>
            {loadingMore() ? t("log_loading_more") : t("log_load_more").replace("{n}", String(total() - logs().length))}
          </Button>
        </div>
      </Show>
    </div>
  );
}
