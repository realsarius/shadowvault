import { createSignal, createMemo, For, Show } from "solid-js";
import { TbOutlineFolderOpen } from "solid-icons/tb";
import { SourceCard } from "./SourceCard";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import { store } from "../../store";
import type { Source } from "../../store/types";
import styles from "./SourceList.module.css";

const FREE_LIMIT = 3;

type SortKey = "name" | "last_run" | "status";
type FilterKey = "all" | "active" | "disabled" | "failed";

interface Props {
  sources: Source[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onEdit?: (id: string) => void;
}

function lastRunOf(s: Source): number {
  const times = s.destinations.map((d) => d.last_run ? new Date(d.last_run).getTime() : 0);
  return times.length ? Math.max(...times) : 0;
}

function hasFailed(s: Source): boolean {
  return s.destinations.some((d) => d.last_status === "Failed");
}

export function SourceList(props: Props) {
  const isLicensed = () => store.licenseStatus === "valid";
  const atLimit = () => !isLicensed() && props.sources.length >= FREE_LIMIT;

  const [sort, setSort] = createSignal<SortKey>("name");
  const [filter, setFilter] = createSignal<FilterKey>("all");

  const processed = createMemo(() => {
    let list = [...props.sources];

    // Filter
    if (filter() === "active") list = list.filter((s) => s.enabled);
    else if (filter() === "disabled") list = list.filter((s) => !s.enabled);
    else if (filter() === "failed") list = list.filter(hasFailed);

    // Sort
    if (sort() === "name") list.sort((a, b) => a.name.localeCompare(b.name));
    else if (sort() === "last_run") list.sort((a, b) => lastRunOf(b) - lastRunOf(a));
    else if (sort() === "status") list.sort((a, b) => (hasFailed(b) ? 1 : 0) - (hasFailed(a) ? 1 : 0));

    return list;
  });

  return (
    <div class={styles.panel}>
      <div class={styles.header}>
        <span class={styles.headerTitle}>{t("src_list_title")}</span>
        <Show
          when={!isLicensed()}
          fallback={<Button size="sm" onClick={props.onAdd}>{t("btn_add_new")}</Button>}
        >
          <div class={styles.headerRight}>
            <span class={atLimit() ? styles.limitReached : styles.limitCounter}>
              {props.sources.length}/{FREE_LIMIT}
            </span>
            <Button size="sm" onClick={props.onAdd}>{t("btn_add_new")}</Button>
          </div>
        </Show>
      </div>

      <div class={styles.toolbar}>
        <select class={styles.toolbarSelect} value={filter()} onChange={(e) => setFilter(e.currentTarget.value as FilterKey)}>
          <option value="all">{t("src_filter_all")}</option>
          <option value="active">{t("src_filter_active")}</option>
          <option value="disabled">{t("src_filter_disabled")}</option>
          <option value="failed">{t("src_filter_failed")}</option>
        </select>
        <select class={styles.toolbarSelect} value={sort()} onChange={(e) => setSort(e.currentTarget.value as SortKey)}>
          <option value="name">{t("src_sort_name")}</option>
          <option value="last_run">{t("src_sort_last_run")}</option>
          <option value="status">{t("src_sort_status")}</option>
        </select>
      </div>

      <div class={styles.list}>
        <Show when={processed().length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}><TbOutlineFolderOpen size={32} /></div>
            {props.sources.length === 0 ? t("src_empty") : t("src_filter_empty")}
            <br />
            <span class={styles.emptyHint}>
              {props.sources.length === 0 ? t("src_empty_hint") : ""}
            </span>
          </div>
        </Show>
        <For each={processed()}>
          {(source) => (
            <div class={styles.itemWrapper}>
              <SourceCard
                source={source}
                active={props.activeId === source.id}
                onClick={() => props.onSelect(source.id)}
                onEdit={() => props.onEdit?.(source.id)}
              />
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
