import { TbOutlineFolder, TbOutlineFile, TbOutlinePencil, TbOutlineFolderOpen } from "solid-icons/tb";
import { Badge } from "../ui/Badge";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import type { Source } from "../../store/types";
import styles from "./SourceCard.module.css";

interface Props {
  source: Source;
  active: boolean;
  onClick: () => void;
  onEdit?: () => void;
}

export function SourceCard(props: Props) {
  return (
    <button
      class={styles.card}
      data-active={String(props.active)}
      onClick={props.onClick}
    >
      <div class={styles.row}>
        <div class={styles.nameRow}>
          <span class={styles.icon}>
            {props.source.source_type === "Directory"
              ? <TbOutlineFolder size={15} />
              : <TbOutlineFile size={15} />}
          </span>
          <span class={styles.name}>{props.source.name}</span>
        </div>
        <div class={styles.cardActions}>
          <Badge variant={props.source.enabled ? "success" : "neutral"}>
            {props.source.enabled ? t("status_active") : t("status_disabled")}
          </Badge>
          <span
            class={styles.editBtn}
            title={t("open_src_folder")}
            role="button"
            tabIndex={0}
            onClick={(e) => { e.stopPropagation(); api.fs.openPath(props.source.path).catch(() => {}); }}
            onKeyDown={(e) => e.key === "Enter" && (e.stopPropagation(), api.fs.openPath(props.source.path).catch(() => {}))}
          >
            <TbOutlineFolderOpen size={12} />
          </span>
          <span
            class={styles.editBtn}
            title={t("btn_edit")}
            role="button"
            tabIndex={0}
            onClick={(e) => { e.stopPropagation(); props.onEdit?.(); }}
            onKeyDown={(e) => e.key === "Enter" && (e.stopPropagation(), props.onEdit?.())}
          >
            <TbOutlinePencil size={12} />
          </span>
        </div>
      </div>
      <div class={styles.path}>{props.source.path}</div>
      <div class={styles.meta}>{props.source.destinations.length} {t("src_targets")}</div>
    </button>
  );
}
