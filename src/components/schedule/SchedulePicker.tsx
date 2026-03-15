import { t } from "../../i18n";
import type { ScheduleType } from "../../store/types";
import styles from "./SchedulePicker.module.css";

const PRO_TYPES: ScheduleType["type"][] = ["Cron", "OnChange"];

interface Props {
  value: ScheduleType;
  onChange: (s: ScheduleType) => void;
  isLicensed?: boolean;
  onProRequired?: () => void;
}

export function SchedulePicker(props: Props) {
  const type = () => props.value.type;

  const isProLocked = (t_: ScheduleType["type"]) =>
    PRO_TYPES.includes(t_) && props.isLicensed === false;

  const setType = (t_: ScheduleType["type"]) => {
    if (isProLocked(t_)) { props.onProRequired?.(); return; }
    if (t_ === "Interval") props.onChange({ type: "Interval", value: { minutes: 60 } });
    else if (t_ === "Cron") props.onChange({ type: "Cron", value: { expression: "0 2 * * *" } });
    else if (t_ === "OnChange") props.onChange({ type: "OnChange" });
    else props.onChange({ type: "Manual" });
  };

  const intervalMinutes = () => (props.value.type === "Interval" ? props.value.value.minutes : 60);
  const cronExpr = () => (props.value.type === "Cron" ? props.value.value.expression : "0 2 * * *");

  return (
    <div class={styles.picker}>
      {(["Interval", "Cron", "OnChange", "Manual"] as const).map((tp) => (
        <label class={`${styles.option} ${isProLocked(tp) ? styles.optionLocked : ""}`}>
          <input type="radio" checked={type() === tp} onChange={() => setType(tp)} />
          {tp === "Interval" && t("schedule_interval")}
          {tp === "Cron" && <>{t("schedule_cron")} {isProLocked(tp) && <span class={styles.proBadge}>Pro</span>}</>}
          {tp === "OnChange" && <>{t("schedule_onchange")} {isProLocked(tp) && <span class={styles.proBadge}>Pro</span>}</>}
          {tp === "Manual" && t("schedule_manual")}
          {tp === "Interval" && type() === "Interval" && (
            <input
              class={styles.numberInput}
              type="number"
              min={1}
              value={intervalMinutes()}
              onInput={(e) => props.onChange({ type: "Interval", value: { minutes: parseInt(e.currentTarget.value) || 60 } })}
            />
          )}
          {tp === "Cron" && type() === "Cron" && (
            <>
              <input
                class={styles.cronInput}
                type="text"
                value={cronExpr()}
                onInput={(e) => props.onChange({ type: "Cron", value: { expression: e.currentTarget.value } })}
                placeholder="0 2 * * *"
              />
              <div class={styles.helpWrap}>
                <button class={styles.helpBtn} type="button">?</button>
                <div class={styles.tooltip}>{t("cron_help_tooltip")}</div>
              </div>
            </>
          )}
        </label>
      ))}
    </div>
  );
}
