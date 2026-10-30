import React, {
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import {
  consumePanelNavigation,
  subscribePanelNavigation,
} from "./navigation.js";
import { api, mutateTask } from "./transport.js";
import { useTaskStream } from "./useTaskStream.js";
import {
  terminal,
  names,
  date,
  elapsed,
  command,
  taskTitle,
  ruleSummary,
} from "./view.js";
import { Icon, Badge } from "./components/Icons.jsx";
import { TaskDetail } from "./components/TaskDetail.jsx";
import { RuntimeStatus } from "./components/RuntimeStatus.jsx";
const initialNavigation = consumePanelNavigation();
const compactLayout = matchMedia("(max-width: 1250px)");
const subscribeLayout = (listener) => {
  compactLayout.addEventListener("change", listener);
  return () => compactLayout.removeEventListener("change", listener);
};
export default function App() {
  const compact = useSyncExternalStore(
    subscribeLayout,
    () => compactLayout.matches,
  );
  const [navigation, setNavigation] = useState(initialNavigation);
  const { token } = navigation;
  const [navExpanded, setNavExpanded] = useState(false),
    [navHovered, setNavHovered] = useState(false);
  const navOpen = navExpanded || navHovered;
  function navigate(next) {
    setPage(next);
    setNavExpanded(false);
    setNavHovered(false);
    if (next === "alerts") setTab("events");
  }
  const [selected, setSelected] = useState(initialNavigation.jobId),
    [tab, setTab] = useState("output"),
    [streamName, setStreamName] = useState("stdout"),
    [filter, setFilter] = useState(""),
    [status, setStatus] = useState("all"),
    [page, setPage] = useState("tasks"),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [form, setForm] = useState(null),
    [grace, setGrace] = useState(5),
    [older, setOlder] = useState([]),
    [cursor, setCursor] = useState(null),
    [more, setMore] = useState(true);
  useEffect(
    () =>
      subscribePanelNavigation((next) => {
        setNavigation(next);
        setSelected(next.jobId);
        setDetail(null);
        setForm(null);
        setError("");
        setFilter("");
        setStatus("all");
        setPage("tasks");
        setTab("output");
      }),
    [],
  );
  const {
    overview,
    detail,
    setDetail,
    connection,
    error: connectionError,
    clearError: clearConnectionError,
  } = useTaskStream(selected, token, navigation);
  const selection = useRef(selected);
  selection.current = selected;

  const jobs = [
    ...new Map(
      [
        ...(detail?.job?.job_id === selected ? [detail.job] : []),
        ...older,
        ...(overview?.jobs || []),
        ...(overview?.active_jobs || []),
      ].map((j) => [j.job_id, j]),
    ).values(),
  ].sort(
    (a, b) =>
      Number(!terminal.has(b.status)) - Number(!terminal.has(a.status)) ||
      b.sequence - a.sequence,
  );
  const displayed = jobs.filter(
    (j) =>
      (status === "all" ||
        (status === "active"
          ? !terminal.has(j.status)
          : j.status === status)) &&
      `${taskTitle(j)} ${command(j)} ${j.cwd} ${j.job_id}`
        .toLowerCase()
        .includes(filter.toLowerCase()),
  );
  async function mutate(body) {
    setBusy(true);
    setError("");
    try {
      await mutateTask(body, token, {
        isCurrent: (id) => selection.current === id,
        onRules: (value) => {
          setDetail((d) =>
            d?.job?.job_id === body.job_id ? { ...d, rules: value.rules } : d,
          );
          setForm(null);
        },
      });
      return true;
    } catch (e) {
      setError(e.message);
      return false;
    } finally {
      setBusy(false);
    }
  }
  function choose(id) {
    setSelected(id);
    setDetail(null);
    setForm(null);
    setTab(page === "alerts" ? "events" : "output");
  }
  function closeDetail() {
    setSelected(null);
    setDetail(null);
    setForm(null);
  }
  async function loadMore() {
    setBusy(true);
    try {
      const before = cursor ?? overview?.next_cursor;
      const result = await api({ op: "inspect", before, limit: 50 }, token);
      setOlder((old) => [...old, ...result.jobs]);
      setCursor(result.next_cursor);
      setMore(result.jobs.length === 50);
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }
  const detailPanel = (
    <TaskDetail
      selected={selected}
      detail={detail}
      busy={busy}
      tab={tab}
      streamName={streamName}
      form={form}
      grace={grace}
      mutate={mutate}
      setForm={setForm}
      setStreamName={setStreamName}
      setTab={setTab}
      setGrace={setGrace}
      closeDetail={closeDetail}
    />
  );

  return (
    <div className="app">
      <aside
        className={`sidebar ${navOpen ? "expanded" : ""}`}
        aria-label="工作空间导航"
        onPointerEnter={(e) => {
          if (e.pointerType === "mouse") setNavHovered(true);
        }}
        onPointerLeave={() => setNavHovered(false)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setNavExpanded(false);
            setNavHovered(false);
            e.currentTarget.querySelector(".nav-toggle")?.focus();
          }
        }}
      >
        <a
          className="brand"
          href="/"
          aria-label="AgentJobRelay · 任务概览"
          onClick={(e) => {
            e.preventDefault();
            navigate("tasks");
          }}
        >
          <img src="/icon.svg" alt="" />
          <span>
            AgentJobRelay<small>LOCAL WORKSPACE</small>
          </span>
        </a>
        <button
          className="icon-button nav-toggle"
          aria-label={navOpen ? "收起导航" : "展开导航"}
          aria-expanded={navOpen}
          aria-controls="workspace-nav"
          onClick={() => {
            if (navOpen) {
              setNavExpanded(false);
              setNavHovered(false);
            } else setNavExpanded(true);
          }}
        >
          <Icon name={navOpen ? "close" : "menu"} />
        </button>
        <div className="nav-caption">工作空间</div>
        <nav id="workspace-nav" aria-label="工作空间">
          <button
            aria-label="任务概览"
            aria-current={page === "tasks" ? "page" : undefined}
            className={page === "tasks" ? "selected" : ""}
            onClick={() => navigate("tasks")}
          >
            <Icon name="grid" />
            <span className="nav-label">任务概览</span>
            <span className="nav-count">
              {overview?.active_jobs?.length || 0}
            </span>
          </button>
          <button
            aria-label="告警与事件"
            aria-current={page === "alerts" ? "page" : undefined}
            className={page === "alerts" ? "selected" : ""}
            onClick={() => navigate("alerts")}
          >
            <Icon name="bell" />
            <span className="nav-label">告警与事件</span>
          </button>
          <button
            aria-label="运行时状态"
            aria-current={page === "runtime" ? "page" : undefined}
            className={page === "runtime" ? "selected" : ""}
            onClick={() => navigate("runtime")}
          >
            <Icon name="pulse" />
            <span className="nav-label">运行时状态</span>
          </button>
        </nav>
        <div className="sidebar-note">
          <Icon name="shield" />
          <strong>本机独立运行</strong>
          <p>监控由运行器执行。关闭浏览器或聊天，任务与规则继续运行。</p>
        </div>
        <footer>
          <span className={`connection-dot ${connection}`} />
          <span className="connection-text">
            {connection === "connected"
              ? "已连接本地运行时"
              : connection === "connecting"
                ? "正在连接…"
                : "连接已中断"}
          </span>
          <small>v{overview?.health?.version || "—"}</small>
        </footer>
      </aside>
      {navExpanded && compact && (
        <button
          className="nav-backdrop"
          aria-label="收起导航"
          onClick={() => setNavExpanded(false)}
        />
      )}

      <main>
        <header>
          <div>
            <div className="eyebrow">
              工作空间 /{" "}
              {page === "runtime"
                ? "运行时"
                : page === "alerts"
                  ? "事件记录"
                  : "任务"}
            </div>
            <h1>
              {page === "runtime"
                ? "运行时状态"
                : page === "alerts"
                  ? "告警与事件"
                  : "任务概览"}
            </h1>
            <p>查看执行进度，设置规则，在需要时停止任务。</p>
          </div>
          <div className="live-label">
            <span className={`connection-dot ${connection}`} />
            {connection === "connected" ? "实时更新" : "等待连接"}
          </div>
        </header>
        {(error || connectionError) && (
          <div className="error" role="alert">
            <span>{error || connectionError}</span>
            <button
              className="icon-button"
              aria-label="关闭提示"
              onClick={() => {
                setError("");
                clearConnectionError();
              }}
            >
              <Icon name="close" />
            </button>
          </div>
        )}
        {!token && (
          <div className="error">
            请通过终端中的 ui 命令打开带访问凭据的本地面板。
          </div>
        )}
        {page === "runtime" ? (
          <RuntimeStatus connection={connection} overview={overview} />
        ) : (
          <>
            <div className="stats">
              <div>
                <span>
                  正在运行
                  <Icon name="pulse" />
                </span>
                <strong>{overview?.active_jobs?.length ?? "—"}</strong>
                <small>独立于聊天和浏览器</small>
              </div>
              <div>
                <span>
                  最近已完成
                  <Icon name="check" />
                </span>
                <strong>
                  {jobs.filter((j) => j.status === "completed").length}
                </strong>
                <small>当前加载的任务</small>
              </div>
              <div>
                <span>
                  需要检查
                  <Icon name="bell" />
                </span>
                <strong>
                  {
                    jobs.filter((j) =>
                      ["failed", "timeout", "orphaned"].includes(j.status),
                    ).length
                  }
                </strong>
                <small>失败、超时或结果未知</small>
              </div>
              <div>
                <span>
                  通知待处理
                  <Icon name="clock" />
                </span>
                <strong>{overview?.health?.retrying_deliveries ?? "—"}</strong>
                <small>持久化投递队列</small>
              </div>
            </div>
            <div className={`workspace ${selected ? "with-detail" : ""}`}>
              <section className="panel task-panel">
                <div className="section-heading">
                  <h2>
                    任务<span className="count">{jobs.length}</span>
                  </h2>
                  <span className="muted small">
                    {date(overview?.sampled_at)}
                  </span>
                </div>
                <div className="toolbar">
                  <div className="search">
                    <Icon name="search" />
                    <input
                      aria-label="搜索任务"
                      placeholder="搜索名称、命令、目录或任务 ID…"
                      value={filter}
                      onChange={(e) => setFilter(e.target.value)}
                    />
                  </div>
                  <select
                    aria-label="筛选任务状态"
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                  >
                    <option value="all">所有状态</option>
                    <option value="active">运行中的任务</option>
                    <option value="completed">已完成</option>
                    <option value="failed">失败</option>
                    <option value="cancelled">已停止</option>
                    <option value="timeout">超时</option>
                    <option value="orphaned">结果未知</option>
                  </select>
                </div>
                {page === "alerts" && (
                  <div className="recent-alerts">
                    <h3>最近告警</h3>
                    {!overview?.recent_alerts?.length && (
                      <p className="muted small">
                        当前热记录中暂无告警；归档任务的完整事件可在任务详情中查看。
                      </p>
                    )}
                    {overview?.recent_alerts?.map((e) => (
                      <button key={e.event_id} onClick={() => choose(e.job_id)}>
                        <Icon name="bell" size={16} />
                        <span>
                          <strong>
                            {e.payload?.rule
                              ? ruleSummary(e.payload.rule)
                              : "规则命中"}
                          </strong>
                          <small>
                            {e.job_id.slice(0, 8)} · {date(e.at)}
                          </small>
                        </span>
                        <Icon name="arrow" size={14} />
                      </button>
                    ))}
                  </div>
                )}
                <div className="task-table">
                  <div className="table-heading">
                    <span>任务 / 工作目录</span>
                    <span>状态</span>
                    <span>运行时长</span>
                  </div>
                  {displayed.map((j) => (
                    <React.Fragment key={j.job_id}>
                      <button
                        className={`task-row ${selected === j.job_id ? "chosen" : ""}`}
                        aria-expanded={
                          compact ? selected === j.job_id : undefined
                        }
                        aria-controls={
                          selected === j.job_id ? "task-detail" : undefined
                        }
                        aria-label={`${taskTitle(j)} · ${names[j.status] || j.status}`}
                        onClick={() =>
                          compact && selected === j.job_id
                            ? closeDetail()
                            : choose(j.job_id)
                        }
                      >
                        <span className="task-title">
                          <span className="command-icon">
                            <Icon name="terminal" />
                          </span>
                          <span>
                            <strong title={taskTitle(j)}>{taskTitle(j)}</strong>
                            <small title={j.cwd}>{j.cwd}</small>
                            <code>{j.job_id.slice(0, 8)}</code>
                          </span>
                        </span>
                        <Badge status={j.status} />
                        <span className="duration">{elapsed(j)}</span>
                        <span className="task-chevron">
                          <Icon name="arrow" size={15} />
                        </span>
                      </button>
                      {compact && selected === j.job_id && detailPanel}
                    </React.Fragment>
                  ))}
                </div>
                {!displayed.length && (
                  <div className="empty">
                    <Icon name="terminal" size={32} />
                    <h3>
                      {jobs.length ? "没有匹配的任务" : "这里会显示后台任务"}
                    </h3>
                    <p>
                      {jobs.length
                        ? "试试其他搜索词或状态。"
                        : "通过插件或终端提交任务后，即可在这里检查和管理。"}
                    </p>
                  </div>
                )}
                {jobs.length > 0 && more && (
                  <div className="table-footer">
                    <button
                      className="button"
                      disabled={busy || !overview?.next_cursor}
                      onClick={loadMore}
                    >
                      加载更早的任务
                    </button>
                  </div>
                )}
              </section>
              {!compact && detailPanel}
            </div>
          </>
        )}
        <div className="bottom-note">
          <Icon name="shield" size={14} />
          <span>仅本机访问 · 状态与规则持久保存 · 查看器关闭不影响任务</span>
        </div>
      </main>
    </div>
  );
}
