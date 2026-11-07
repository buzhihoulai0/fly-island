import { useTranslation } from "../../i18n";
import {
  formatBytes,
  formatCount,
  formatLoad,
  formatPercent,
  formatRate,
  formatTemperature,
  formatUptime,
  percentOf,
} from "./format";
import { getHostPulseState } from "./status";
import type { HostMonitorService, HostMonitorView } from "./types";
import "./HostMonitorPanel.css";

const SPARK_WIDTH = 96;
const SPARK_HEIGHT = 20;

type HostMonitorPanelProps = {
  view: HostMonitorView | null;
  onRetry: () => void;
  onOpenSettings: () => void;
};

/** 展开页：小主机的 CPU / 内存 / 磁盘 / 温度与服务列表。 */
export function HostMonitorPanel({
  view,
  onRetry,
  onOpenSettings,
}: HostMonitorPanelProps) {
  const t = useTranslation();
  const pulse = getHostPulseState(view);
  const payload = view?.payload ?? null;

  if (!view || !view.configured) {
    return (
      <div className="host-panel host-panel--empty">
        <p className="host-panel__empty-title">{t.host.notConfigured}</p>
        <p className="host-panel__empty-hint">{t.host.notConfiguredHint}</p>
        <button className="host-panel__action" type="button" onClick={onOpenSettings}>
          {t.host.openSettings}
        </button>
      </div>
    );
  }

  if (!payload) {
    return (
      <div className="host-panel host-panel--empty">
        <p className="host-panel__empty-title">
          {view.connected ? t.host.connecting : t.host.offline}
        </p>
        <p className="host-panel__empty-hint">{view.error ?? t.host.noData}</p>
        <button className="host-panel__action" type="button" onClick={onRetry}>
          {t.host.retry}
        </button>
      </div>
    );
  }

  const sys = payload.sys;
  const history = payload.history;
  const counts = payload.counts;
  const hostName = payload.host?.name ?? sys?.hostname ?? stripProtocol(view.baseUrl);
  const cores = sys?.cores ?? payload.host?.cores ?? null;
  const updatedSeconds = Math.max(0, Math.round((Date.now() - view.fetchedAt) / 1000));

  return (
    <div className="host-panel">
      <header className="host-panel__head">
        <div className="host-panel__identity">
          <span className="host-panel__name">{hostName}</span>
          <span className="host-panel__address">{stripProtocol(view.baseUrl)}</span>
        </div>
        <span className={`host-panel__badge host-panel__badge--${pulse}`}>
          {view.stale ? t.host.stale : t.host.pulse[pulse]}
        </span>
      </header>

      <div className="host-panel__metrics">
        <article className="host-panel__metric host-panel__metric--cpu">
          <div className="host-panel__metric-head">
            <span>{t.host.cpu}</span>
            <span className="host-panel__metric-detail">
              {formatCount(cores)} {t.host.cores}
            </span>
          </div>
          <strong className="host-panel__metric-value">{formatPercent(sys?.cpu)}</strong>
          <Sparkline series={history?.cpu} />
        </article>

        <article className="host-panel__metric host-panel__metric--memory">
          <div className="host-panel__metric-head">
            <span>{t.host.memory}</span>
            <span className="host-panel__metric-detail">
              {formatBytes(sys?.memUsed)} / {formatBytes(sys?.memTotal)}
            </span>
          </div>
          <strong className="host-panel__metric-value">{formatPercent(sys?.memPct)}</strong>
          <Sparkline series={history?.mem} />
        </article>

        <article className="host-panel__metric host-panel__metric--disk">
          <div className="host-panel__metric-head">
            <span>{t.host.disk}</span>
            <span className="host-panel__metric-detail">
              {formatBytes(sys?.diskUsed)} / {formatBytes(sys?.diskTotal)}
            </span>
          </div>
          <strong className="host-panel__metric-value">{formatPercent(sys?.diskPct)}</strong>
          <div className="host-panel__bar" aria-hidden="true">
            <span style={{ width: `${percentOf(sys?.diskUsed, sys?.diskTotal)}%` }} />
          </div>
        </article>

        <article className="host-panel__metric host-panel__metric--temperature">
          <div className="host-panel__metric-head">
            <span>{t.host.temperature}</span>
            <span className="host-panel__metric-detail">
              {t.host.load} {formatLoad(sys?.load)}
            </span>
          </div>
          <strong className="host-panel__metric-value">{formatTemperature(sys?.temp)}</strong>
          <span className="host-panel__uptime">
            {t.host.uptime} {formatUptime(sys?.uptime)}
          </span>
        </article>
      </div>

      <div className="host-panel__body">
        <section className="host-panel__card host-panel__services">
          <header className="host-panel__card-head">
            <span>{t.host.services}</span>
            <span className="host-panel__card-detail">
              {counts
                ? t.host.servicesSummary(counts.ok, counts.total, counts.degraded + counts.down)
                : t.host.noData}
            </span>
          </header>
          <ul className="host-panel__service-list">
            {payload.services.map((service) => (
              <ServiceRow key={service.id || service.name} service={service} />
            ))}
          </ul>
        </section>

        <section className="host-panel__card host-panel__side">
          <header className="host-panel__card-head">
            <span>{t.host.network}</span>
            <span className="host-panel__card-detail">
              {t.host.refreshInterval(Math.round(view.intervalMs / 1000))}
            </span>
          </header>
          <Sparkline series={history?.netRx} tone="network" />
          <dl className="host-panel__stats">
            <div>
              <dt>{t.host.download}</dt>
              <dd>{formatRate(sys?.rxRate)}</dd>
            </div>
            <div>
              <dt>{t.host.upload}</dt>
              <dd>{formatRate(sys?.txRate)}</dd>
            </div>
            <div>
              <dt>{t.host.processes}</dt>
              <dd>
                {formatCount(payload.procs?.total)} · {formatBytes(payload.procs?.rss)}
              </dd>
            </div>
            <div>
              <dt>{t.host.load}</dt>
              <dd>{formatLoad(sys?.load)}</dd>
            </div>
            <div>
              <dt>{t.host.updated}</dt>
              <dd>{t.host.updatedAgo(updatedSeconds)}</dd>
            </div>
          </dl>
        </section>
      </div>
    </div>
  );
}

function ServiceRow({ service }: { service: HostMonitorService }) {
  const t = useTranslation();
  const status =
    service.status === "ok" || service.status === "degraded" || service.status === "down"
      ? service.status
      : "unknown";
  const ports = service.ports
    .filter((port) => port.open)
    .map((port) => `:${port.port}`)
    .join(" ");
  const meta = [ports, service.http?.status ? `HTTP ${service.http.status}` : ""]
    .filter(Boolean)
    .join(" · ");

  return (
    <li
      className={`host-panel__service host-panel__service--${status}${
        service.optional ? " host-panel__service--optional" : ""
      }`}
    >
      <span className="host-panel__service-dot" aria-hidden="true" />
      <span className="host-panel__service-name">{service.name}</span>
      <span className="host-panel__service-usage">
        {service.status === "down"
          ? t.host.status.down
          : service.cpu !== null
            ? `cpu ${formatCount(service.cpu)} · ${formatBytes(service.rss)}`
            : meta || service.note || ""}
      </span>
    </li>
  );
}

function Sparkline({
  series,
  tone = "primary",
}: {
  series: number[] | undefined;
  tone?: "primary" | "network";
}) {
  const points = buildPoints(series);

  if (!points) {
    return (
      <div
        className={`host-panel__spark host-panel__spark--${tone} host-panel__spark--empty`}
        aria-hidden="true"
      />
    );
  }

  return (
    <svg
      className={`host-panel__spark host-panel__spark--${tone}`}
      viewBox={`0 0 ${SPARK_WIDTH} ${SPARK_HEIGHT}`}
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <polyline points={points} fill="none" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

function stripProtocol(baseUrl: string) {
  const separator = baseUrl.indexOf("://");
  return separator === -1 ? baseUrl : baseUrl.slice(separator + 3);
}

function buildPoints(series: number[] | undefined) {
  if (!series) {
    return null;
  }

  const values = series.filter((value) => Number.isFinite(value));
  if (values.length < 2) {
    return null;
  }

  const max = Math.max(...values, 1);
  const step = SPARK_WIDTH / (values.length - 1);

  return values
    .map((value, index) => {
      const x = index * step;
      const y = SPARK_HEIGHT - 1 - (value / max) * (SPARK_HEIGHT - 2);
      return `${x.toFixed(1)},${Math.max(0, Math.min(SPARK_HEIGHT, y)).toFixed(1)}`;
    })
    .join(" ");
}
