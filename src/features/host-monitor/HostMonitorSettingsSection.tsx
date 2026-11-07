import { useCallback, useEffect, useRef, useState } from "react";
import { NumberControl } from "../../components/controls/NumberControl";
import { ToggleControl } from "../../components/controls/ToggleControl";
import { useTranslation } from "../../i18n";
import { getErrorMessage } from "../../utils/error";
import { setHostMonitorConfig, testHostMonitorConnection } from "./api";
import { useHostMonitor } from "./useHostMonitor";
import "./HostMonitorSettingsSection.css";

type TestState =
  | { kind: "idle" }
  | { kind: "testing" }
  | { kind: "ok"; message: string }
  | { kind: "error"; message: string };

/** 设置窗口「集成」页的小主机监控配置。 */
export function HostMonitorSettingsSection() {
  const t = useTranslation();
  const { view } = useHostMonitor();
  const [baseUrl, setBaseUrl] = useState("");
  const [intervalSeconds, setIntervalSeconds] = useState(3);
  const [enabled, setEnabled] = useState(true);
  const [showInCollapsed, setShowInCollapsed] = useState(true);
  const [token, setToken] = useState("");
  const [hasToken, setHasToken] = useState(false);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved">("idle");
  const [testState, setTestState] = useState<TestState>({ kind: "idle" });
  const syncedSignatureRef = useRef("");

  // 视图每次轮询都会刷新；只有配置字段真的变了才覆盖正在编辑的草稿。
  useEffect(() => {
    if (!view) {
      return;
    }

    const signature = [
      view.baseUrl,
      view.intervalMs,
      view.enabled,
      view.showInCollapsed,
      view.hasToken,
    ].join("|");

    if (signature === syncedSignatureRef.current) {
      return;
    }

    syncedSignatureRef.current = signature;
    setBaseUrl(view.baseUrl);
    setIntervalSeconds(Math.max(1, Math.round(view.intervalMs / 1000)));
    setEnabled(view.enabled);
    setShowInCollapsed(view.showInCollapsed);
    setHasToken(view.hasToken);
  }, [view]);

  const readTokenDraft = useCallback(() => {
    const trimmed = token.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }, [token]);

  const handleSave = useCallback(async () => {
    setSaveState("saving");
    setTestState({ kind: "idle" });
    const draftToken = readTokenDraft();

    try {
      await setHostMonitorConfig({
        baseUrl,
        intervalMs: intervalMsFor(intervalSeconds),
        enabled,
        showInCollapsed,
        ...(draftToken === undefined ? {} : { token: draftToken }),
      });
      setToken("");
      syncedSignatureRef.current = "";
      setSaveState("saved");
    } catch (error) {
      setSaveState("idle");
      setTestState({ kind: "error", message: getErrorMessage(error) });
    }
  }, [baseUrl, enabled, intervalSeconds, readTokenDraft, showInCollapsed]);

  const handleClearToken = useCallback(async () => {
    try {
      await setHostMonitorConfig({
        baseUrl,
        intervalMs: intervalMsFor(intervalSeconds),
        enabled,
        showInCollapsed,
        token: "",
      });
      setToken("");
      setHasToken(false);
      syncedSignatureRef.current = "";
    } catch (error) {
      setTestState({ kind: "error", message: getErrorMessage(error) });
    }
  }, [baseUrl, enabled, intervalSeconds, showInCollapsed]);

  const handleTest = useCallback(async () => {
    setTestState({ kind: "testing" });

    try {
      const result = await testHostMonitorConnection(
        baseUrl.trim(),
        readTokenDraft() ?? (hasToken ? null : ""),
      );

      setTestState(
        result.ok
          ? {
              kind: "ok",
              message: t.settings.hostMonitor.testOk(
                result.hostname ?? baseUrl,
                result.latencyMs,
                result.serviceCount,
              ),
            }
          : { kind: "error", message: t.settings.hostMonitor.testFailed(result.message) },
      );
    } catch (error) {
      setTestState({
        kind: "error",
        message: t.settings.hostMonitor.testFailed(getErrorMessage(error)),
      });
    }
  }, [baseUrl, hasToken, readTokenDraft, t]);

  const statusText =
    saveState === "saving"
      ? t.settings.hostMonitor.saving
      : saveState === "saved"
        ? t.settings.hostMonitor.saved
        : "";

  return (
    <section className="settings-section settings-section--host-monitor">
      <div className="settings-section__header">
        <span>{t.settings.hostMonitor.title}</span>
      </div>
      <p className="host-settings__description">{t.settings.hostMonitor.description}</p>

      <ToggleControl
        label={t.settings.hostMonitor.enabled}
        checked={enabled}
        onChange={setEnabled}
      />
      <ToggleControl
        label={t.settings.hostMonitor.showInCollapsed}
        checked={showInCollapsed}
        onChange={setShowInCollapsed}
      />

      <label className="host-settings__field">
        <span className="host-settings__label">{t.settings.hostMonitor.baseUrl}</span>
        <input
          className="host-settings__input"
          type="text"
          spellCheck={false}
          value={baseUrl}
          placeholder={t.settings.hostMonitor.baseUrlPlaceholder}
          onChange={(event) => setBaseUrl(event.currentTarget.value)}
        />
      </label>

      <NumberControl
        label={t.settings.hostMonitor.interval}
        value={intervalSeconds}
        min={1}
        max={30}
        onChange={setIntervalSeconds}
      />
      <p className="host-settings__hint">{t.settings.hostMonitor.intervalHint}</p>

      <label className="host-settings__field">
        <span className="host-settings__label">{t.settings.hostMonitor.token}</span>
        <input
          className="host-settings__input"
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={token}
          placeholder={hasToken ? t.settings.hostMonitor.tokenSaved(true) : t.settings.hostMonitor.tokenPlaceholder}
          onChange={(event) => setToken(event.currentTarget.value)}
        />
      </label>
      <p className="host-settings__hint">{t.settings.hostMonitor.tokenHint}</p>

      <div className="host-settings__actions">
        <button className="host-settings__button" type="button" onClick={() => void handleSave()}>
          {saveState === "saving" ? t.settings.hostMonitor.saving : t.settings.hostMonitor.save}
        </button>
        <button className="host-settings__button host-settings__button--ghost" type="button" onClick={() => void handleTest()}>
          {testState.kind === "testing" ? t.settings.hostMonitor.testing : t.settings.hostMonitor.test}
        </button>
        {hasToken && (
          <button
            className="host-settings__button host-settings__button--ghost"
            type="button"
            onClick={() => void handleClearToken()}
          >
            {t.settings.hostMonitor.tokenClear}
          </button>
        )}
        {statusText !== "" && <span className="host-settings__status">{statusText}</span>}
      </div>

      {testState.kind === "ok" && (
        <p className="host-settings__status host-settings__status--ok">{testState.message}</p>
      )}
      {testState.kind === "error" && (
        <p className="host-settings__status host-settings__status--error">{testState.message}</p>
      )}
    </section>
  );
}

/** 设置窗口以秒为单位，配置以毫秒存储（1s–30s），这里收敛非法输入。 */
function intervalMsFor(seconds: number) {
  const safeSeconds = Number.isFinite(seconds) ? Math.round(seconds) : 3;
  return Math.min(30, Math.max(1, safeSeconds)) * 1_000;
}
