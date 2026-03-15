import { For, Show } from "solid-js";
import { SourceCard } from "./SourceCard";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import type { Source } from "../../store/types";
import styles from "./SourceList.module.css";

interface Props {
  sources: Source[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
}

export function SourceList(props: Props) {
  return (
    <div class={styles.panel}>
      <div class={styles.header}>
        <span class={styles.headerTitle}>{t("src_list_title")}</span>
        <Button size="sm" onClick={props.onAdd}>{t("btn_add_new")}</Button>
      </div>
      <div class={styles.list}>
        <Show when={props.sources.length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}>📂</div>
            {t("src_empty")}
            <br />
            <span class={styles.emptyHint}>{t("src_empty_hint")}</span>
          </div>
        </Show>
        <For each={props.sources}>
          {(source) => (
            <div class={styles.itemWrapper}>
              <SourceCard
                source={source}
                active={props.activeId === source.id}
                onClick={() => props.onSelect(source.id)}
              />
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
