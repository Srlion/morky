import { formatDistanceToNow, isValid, parseISO } from "date-fns";

export function fmtDate(d) {
  if (!d) return "-";
  return new Date(d).toLocaleDateString("en-GB", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

export function fmtDateTime(d) {
  if (!d) return "-";
  return new Date(d).toLocaleString("en-GB", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

export function fmtRelativeTime(d) {
  if (!d) return "-";

  const date =
    d instanceof Date ? d : typeof d === "string" ? parseISO(d) : new Date(d);

  if (!isValid(date)) return "-";
  return formatDistanceToNow(date, { addSuffix: true });
}
