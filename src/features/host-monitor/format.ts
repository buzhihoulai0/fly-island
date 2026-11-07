const SIZE_UNITS = ["B", "K", "M", "G", "T"] as const;

export function formatPercent(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }

  return `${Math.round(value)}%`;
}

export function formatBytes(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value) || value < 0) {
    return "—";
  }

  if (value < 1024) {
    return `${Math.round(value)} B`;
  }

  let size = value;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < SIZE_UNITS.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }

  const rounded = size >= 10 ? Math.round(size) : Math.round(size * 10) / 10;
  return `${rounded} ${SIZE_UNITS[unitIndex]}`;
}

export function formatRate(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }

  return `${formatBytes(value)}/s`;
}

export function formatTemperature(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value) || value <= 0) {
    return "—";
  }

  return `${Math.round(value)}°C`;
}

export function formatLoad(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }

  return value.toFixed(2);
}

export function formatCount(value: number | null | undefined) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }

  return String(Math.round(value));
}

export function formatUptime(seconds: number | null | undefined) {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds) || seconds <= 0) {
    return "—";
  }

  const totalMinutes = Math.floor(seconds / 60);
  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;

  if (days > 0) {
    return `${days}d ${hours}h`;
  }

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  return `${minutes}m`;
}

/** 小主机返回的是以秒计的 age/uptime 相对值。 */
export function formatAgo(seconds: number | null | undefined) {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) {
    return "—";
  }

  const safe = Math.max(0, Math.round(seconds));

  if (safe < 60) {
    return `${safe}s`;
  }

  if (safe < 3600) {
    return `${Math.round(safe / 60)}m`;
  }

  if (safe < 86400) {
    return `${Math.round(safe / 3600)}h`;
  }

  return `${Math.round(safe / 86400)}d`;
}

export function percentOf(part: number | null | undefined, total: number | null | undefined) {
  if (!part || !total || !Number.isFinite(part) || !Number.isFinite(total) || total <= 0) {
    return 0;
  }

  return Math.min(100, Math.max(0, (part / total) * 100));
}
