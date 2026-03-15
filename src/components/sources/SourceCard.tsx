import { Badge } from "../ui/Badge";
import { t } from "../../i18n";
import type { Source } from "../../store/types";
import styles from "./SourceCard.module.css";

interface Props {
  source: Source;
  active: boolean;
  onClick: () => void;
}

export function SourceCard(props: Props) {
  const icon = () => props.source.source_type === "Directory" ? "📁" : "📄";

  return (
    <button
      class={styles.card}
      data-active={String(props.active)}
      onClick={props.onClick}
    >
      <div class={styles.row}>
        <div class={styles.nameRow}>
          <span class={styles.icon}>{icon()}</span>
          <span class={styles.name}>{props.source.name}</span>
        </div>
        <Badge variant={props.source.enabled ? "success" : "neutral"}>
          {props.source.enabled ? t("status_active") : t("status_disabled")}
        </Badge>
      </div>
      <div class={styles.path}>{props.source.path}</div>
      <div class={styles.meta}>{props.source.destinations.length} {t("src_targets")}</div>
    </button>
  );
}
