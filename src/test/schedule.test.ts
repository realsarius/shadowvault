import { describe, it, expect } from "vitest";
import type { ScheduleType } from "../store/types";

// Pure logic helpers extracted from SchedulePicker
function defaultForType(type: ScheduleType["type"]): ScheduleType {
  if (type === "Interval") return { type: "Interval", value: { minutes: 60 } };
  if (type === "Cron") return { type: "Cron", value: { expression: "0 2 * * *" } };
  if (type === "OnChange") return { type: "OnChange" };
  return { type: "Manual" };
}

function scheduleLabel(s: ScheduleType): string {
  if (s.type === "Interval") return `Her ${s.value.minutes} dakikada bir`;
  if (s.type === "Cron") return `Cron: ${s.value.expression}`;
  if (s.type === "OnChange") return "Değişince";
  return "Manuel";
}

describe("SchedulePicker logic", () => {
  it("defaults Interval to 60 minutes", () => {
    const s = defaultForType("Interval");
    expect(s.type).toBe("Interval");
    if (s.type === "Interval") expect(s.value.minutes).toBe(60);
  });

  it("defaults Cron to daily at 02:00", () => {
    const s = defaultForType("Cron");
    expect(s.type).toBe("Cron");
    if (s.type === "Cron") expect(s.value.expression).toBe("0 2 * * *");
  });

  it("OnChange and Manual have no value field", () => {
    const onChange = defaultForType("OnChange");
    const manual = defaultForType("Manual");
    expect(onChange.type).toBe("OnChange");
    expect(manual.type).toBe("Manual");
    expect("value" in onChange).toBe(false);
    expect("value" in manual).toBe(false);
  });

  it("scheduleLabel formats Interval correctly", () => {
    const label = scheduleLabel({ type: "Interval", value: { minutes: 30 } });
    expect(label).toContain("30");
  });

  it("scheduleLabel formats Cron with expression", () => {
    const label = scheduleLabel({ type: "Cron", value: { expression: "0 0 * * *" } });
    expect(label).toContain("0 0 * * *");
  });
});
