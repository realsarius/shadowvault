import { For, Show } from "solid-js";
import { TbOutlineFolderOpen } from "solid-icons/tb";
import { SourceCard } from "./SourceCard";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import { store } from "../../store";
import type { Source } from "../../store/types";
import styles from "./SourceList.module.css";

const FREE_LIMIT = 3;

interface Props {
  sources: Source[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onEdit?: (id: string) => void;
}

export function SourceList(props: Props) {
  const isLicensed = () => store.licenseStatus === "valid";
  const atLimit = () => !isLicensed() && props.sources.length >= FREE_LIMIT;

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
      <div class={styles.list}>
        <Show when={props.sources.length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}><TbOutlineFolderOpen size={32} /></div>
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
                onEdit={() => props.onEdit?.(source.id)}
              />
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
