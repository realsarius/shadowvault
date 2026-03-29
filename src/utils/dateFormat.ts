import { store } from "../store";

/** Returns the effective IANA timezone: user setting or system default. */
function effectiveTz(): string | undefined {
  const tz = store.settings?.timezone;
  if (!tz || tz === "auto") return undefined;
  return tz;
}

/** Formats an ISO datetime string with the app-wide timezone setting applied. */
export function formatDateTime(
  iso: string | null,
  opts: Omit<Intl.DateTimeFormatOptions, "timeZone"> = {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }
): string {
  if (!iso) return "—";
  const tz = effectiveTz();
  return new Date(iso).toLocaleString(undefined, {
    ...opts,
    ...(tz ? { timeZone: tz } : {}),
  });
}

/** Formats an ISO datetime string as date-only (no time). */
export function formatDateOnly(iso: string | null): string {
  if (!iso) return "—";
  const tz = effectiveTz();
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    ...(tz ? { timeZone: tz } : {}),
  });
}

/** All selectable timezones shown in Settings. */
export const TIMEZONE_OPTIONS: { label: string; value: string }[] = [
  { label: "Otomatik (Sistem)", value: "auto" },
  { label: "UTC-12:00 — Baker Island", value: "Etc/GMT+12" },
  { label: "UTC-11:00 — Samoa", value: "Pacific/Midway" },
  { label: "UTC-10:00 — Hawaii", value: "Pacific/Honolulu" },
  { label: "UTC-9:00 — Alaska", value: "America/Anchorage" },
  { label: "UTC-8:00 — Los Angeles, Vancouver", value: "America/Los_Angeles" },
  { label: "UTC-7:00 — Denver, Phoenix", value: "America/Denver" },
  { label: "UTC-6:00 — Chicago, Mexico City", value: "America/Chicago" },
  { label: "UTC-5:00 — New York, Toronto", value: "America/New_York" },
  { label: "UTC-4:00 — Caracas, Santiago", value: "America/Caracas" },
  { label: "UTC-3:00 — São Paulo, Buenos Aires", value: "America/Sao_Paulo" },
  { label: "UTC-2:00 — South Georgia", value: "Atlantic/South_Georgia" },
  { label: "UTC-1:00 — Azores", value: "Atlantic/Azores" },
  { label: "UTC+0:00 — Londra, Dublin", value: "Europe/London" },
  { label: "UTC+1:00 — Paris, Berlin, Roma", value: "Europe/Paris" },
  { label: "UTC+2:00 — Atina, Kahire, Johannesburg", value: "Europe/Athens" },
  { label: "UTC+3:00 — İstanbul, Moskova, Riyad", value: "Europe/Istanbul" },
  { label: "UTC+4:00 — Dubai, Bakü", value: "Asia/Dubai" },
  { label: "UTC+4:30 — Kabil", value: "Asia/Kabul" },
  { label: "UTC+5:00 — Karaçi, İslamabad", value: "Asia/Karachi" },
  { label: "UTC+5:30 — Mumbai, Kolkata", value: "Asia/Kolkata" },
  { label: "UTC+5:45 — Katmandu", value: "Asia/Kathmandu" },
  { label: "UTC+6:00 — Dakka, Almatı", value: "Asia/Dhaka" },
  { label: "UTC+6:30 — Yangon", value: "Asia/Rangoon" },
  { label: "UTC+7:00 — Bangkok, Jakarta", value: "Asia/Bangkok" },
  { label: "UTC+8:00 — Pekin, Singapur", value: "Asia/Singapore" },
  { label: "UTC+9:00 — Tokyo, Seul", value: "Asia/Tokyo" },
  { label: "UTC+9:30 — Adelaide, Darwin", value: "Australia/Adelaide" },
  { label: "UTC+10:00 — Sidney, Melbourne", value: "Australia/Sydney" },
  { label: "UTC+11:00 — Solomon Adaları", value: "Pacific/Guadalcanal" },
  { label: "UTC+12:00 — Auckland, Wellington", value: "Pacific/Auckland" },
  { label: "UTC+13:00 — Apia", value: "Pacific/Apia" },
  { label: "UTC+14:00 — Line Adaları", value: "Pacific/Kiritimati" },
];
