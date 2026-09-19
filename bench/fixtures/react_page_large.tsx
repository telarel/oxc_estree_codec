import * as React from "react";

type BadgeTone = "neutral" | "positive" | "warning" | "critical";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

type Severity = "info" | "minor" | "major" | "severe";

type TrendDirection = "up" | "down" | "flat";

type SortDirection = "ascending" | "descending" | "none";

type ActivityKind = "deploy" | "review" | "comment" | "incident" | "merge";

type ChannelKind = "email" | "slack" | "sms" | "webhook";

type DialogKind = "none" | "confirm" | "notes";

type MilestoneStatus = "planned" | "active" | "shipped";

type ColumnStatus = "backlog" | "todo" | "doing" | "review" | "done";

type RegionKind = "us" | "eu" | "apac";

type TabId = "metrics" | "logs" | "traces";

type Card = {
    id: string;
    title: string;
    points: number;
    assignee: string;
    tags: Array<string>;
};

type ColumnData = {
    status: ColumnStatus;
    wipLimit: number;
    cards: Array<Card>;
};

type NavItem = { id: string; label: string; count: number };

type NavGroup = { id: string; label: string; items: Array<NavItem> };

type StatCard = {
    id: string;
    label: string;
    value: number;
    unit: string;
    trend: TrendDirection;
    delta: number;
    note: string;
};

type TableColumn = { id: string; header: string; sortable: boolean };

type TableRow = {
    id: string;
    service: string;
    region: RegionKind;
    severity: Severity;
    latencyMs: number;
    uptimePct: number;
    owner: string;
    incidents: number;
};

type ActivityEvent = {
    id: string;
    kind: ActivityKind;
    actor: string;
    target: string;
    minutesAgo: number;
    detail: string;
};

type NotificationChannel = {
    id: string;
    kind: ChannelKind;
    label: string;
    description: string;
    enabled: boolean;
    threshold: number;
};

type SettingRow =
    | {
          id: string;
          kind: "toggle";
          label: string;
          enabled: boolean;
      }
    | {
          id: string;
          kind: "choice";
          label: string;
          value: string;
          options: Array<string>;
      };

type Milestone = {
    id: string;
    label: string;
    window: string;
    status: MilestoneStatus;
    progress: number;
    owner: string;
};

type TeamMember = {
    id: string;
    name: string;
    role: string;
    region: RegionKind;
    load: number;
};

type BadgeProps = { label: string; tone: BadgeTone };

type ButtonProps = {
    label: string;
    variant: ButtonVariant;
    disabled: boolean;
    onClick: () => void;
};

type ToggleProps = {
    label: string;
    checked: boolean;
    onChange: (next: boolean) => void;
};

type TextInputProps = {
    value: string;
    placeholder: string;
    onChange: (next: string) => void;
};

type HeaderProps = {
    title: string;
    subtitle: string;
    environment: string;
    onOpenDialog: (kind: DialogKind) => void;
};

type SidebarNavProps = {
    groups: Array<NavGroup>;
    activeId: string;
    onSelect: (id: string) => void;
};

type StatCardTileProps = { card: StatCard };

type StatCardGridProps = { cards: Array<StatCard> };

type CardTileProps = { card: Card; onPromote: (id: string) => void };

type ColumnProps = { column: ColumnData; onPromote: (id: string) => void };

type AddCardFormProps = {
    draft: string;
    onDraftChange: (value: string) => void;
    onSubmit: () => void;
};

type KanbanBoardProps = {
    columns: Array<ColumnData>;
    draft: string;
    onDraftChange: (next: string) => void;
    onAddCard: () => void;
    onPromote: (id: string) => void;
};

type DataTableProps = {
    columns: Array<TableColumn>;
    rows: Array<TableRow>;
    sortId: string;
    direction: SortDirection;
    onSort: (id: string) => void;
    page: number;
    pageCount: number;
    onPageChange: (next: number) => void;
};

type ActivityFeedProps = { events: Array<ActivityEvent> };

type NotificationPreferencesProps = {
    channels: Array<NotificationChannel>;
    onToggle: (id: string, next: boolean) => void;
    onThresholdChange: (id: string, next: number) => void;
};

type SettingsPanelProps = {
    rows: Array<SettingRow>;
    onToggle: (id: string, next: boolean) => void;
    onChoice: (id: string, next: string) => void;
};

type TimelineProps = { milestones: Array<Milestone> };

type TabBarProps = {
    active: TabId;
    labels: Record<TabId, string>;
    onSelect: (id: TabId) => void;
};

type ConfirmDialogProps = {
    open: boolean;
    title: string;
    body: string;
    onCancel: () => void;
    onConfirm: () => void;
};

type NotesDialogProps = {
    open: boolean;
    body: string;
    onClose: () => void;
};

type TeamRosterProps = { members: Array<TeamMember> };

type FooterProps = { note: string };

const columnOrder: Array<ColumnStatus> = [
    "backlog",
    "todo",
    "doing",
    "review",
    "done",
];

const statusLabels: Record<ColumnStatus, string> = {
    backlog: "Backlog",
    todo: "To do",
    doing: "Doing",
    review: "In review",
    done: "Shipped",
};

const toneForLoad = (count: number, wipLimit: number): BadgeTone => {
    if (count >= wipLimit) return "critical";
    if (count === wipLimit - 1) return "warning";
    return "neutral";
};

const toneForSeverity = (severity: Severity): BadgeTone => {
    if (severity === "severe") return "critical";
    if (severity === "major") return "warning";
    if (severity === "minor") return "neutral";
    return "positive";
};

const toneForTrend = (trend: TrendDirection): BadgeTone => {
    if (trend === "up") return "positive";
    if (trend === "down") return "critical";
    return "neutral";
};

const trendSymbol = (trend: TrendDirection): string => {
    if (trend === "up") return "▲";
    if (trend === "down") return "▼";
    return "→";
};

const sortDirectionAfter = (direction: SortDirection): SortDirection => {
    if (direction === "none") return "ascending";
    if (direction === "ascending") return "descending";
    return "none";
};

const pageWindow = (page: number, pageCount: number): Array<number> => {
    const start: number = Math.max(1, page - 1);
    const end: number = Math.min(pageCount, page + 2);
    const window: Array<number> = [];
    let cursor: number = start;
    while (cursor <= end) {
        window.push(cursor);
        cursor = cursor + 1;
    }
    return window;
};

const formatLatency = (latencyMs: number): string => `${latencyMs} ms`;

const formatUptime = (uptimePct: number): string => `${uptimePct.toFixed(2)}%`;

const formatAge = (minutesAgo: number): string => {
    if (minutesAgo < 60) return `${minutesAgo}m ago`;
    const hours: number = Math.floor(minutesAgo / 60);
    if (hours < 24) return `${hours}h ago`;
    const days: number = Math.floor(hours / 24);
    return `${days}d ago`;
};

const activityLabels: Record<ActivityKind, string> = {
    deploy: "Deployed",
    review: "Requested review",
    comment: "Commented on",
    incident: "Opened incident for",
    merge: "Merged",
};

const channelHints: Record<ChannelKind, string> = {
    email: "Digest and incident mail",
    slack: "Channel webhooks",
    sms: "Pager escalations",
    webhook: "Raw JSON events",
};

const regionLabels: Record<RegionKind, string> = {
    us: "US East",
    eu: "EU West",
    apac: "APAC South",
};

const tabLabels: Record<TabId, string> = {
    metrics: "Metrics",
    logs: "Logs",
    traces: "Traces",
};

const pageIntro: string =
    "This dashboard aggregates release health, service reliability and " +
    "team capacity signals. Each section reads from the shared snapshot " +
    "below and dispatches typed callbacks upward across a realistic tree.";

const incidentSummary: string =
    "Two severity-major incidents were opened this week. The eu incident " +
    "was mitigated by rolling back a cache change; the apac incident " +
    "cleared once the reindex queue drained. Both share a postmortem item " +
    "to add synthetic checks before the rollout window opens.";

const capacityNote: string =
    "Team load is calculated from review queues, on-call rotations and open " +
    "work items weighted by estimated points. Load above ninety suggests the " +
    "owner should shed scope this cycle; load below forty suggests capacity " +
    "for a stretch item from the backlog grooming list.";

const Badge = ({ label, tone }: BadgeProps): React.JSX.Element => (
    <span className={`badge badge-${tone}`}>{label}</span>
);

const Button = ({
    label,
    variant,
    disabled,
    onClick,
}: ButtonProps): React.JSX.Element => (
    <button
        type={"button"}
        className={`btn btn-${variant}`}
        disabled={disabled}
        onClick={onClick}
    >
        {label}
    </button>
);

const Toggle = ({
    label,
    checked,
    onChange,
}: ToggleProps): React.JSX.Element => (
    <label className={"toggle-row"}>
        <input
            type={"checkbox"}
            checked={checked}
            onChange={(event) => onChange(event.target.checked)}
        />
        <span className={"toggle-label"}>{label}</span>
        <span className={"toggle-state"}>{checked ? "on" : "off"}</span>
    </label>
);

const TextInput = ({
    value,
    placeholder,
    onChange,
}: TextInputProps): React.JSX.Element => (
    <input
        type={"text"}
        className={"text-input"}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
    />
);

const Header = ({
    title,
    subtitle,
    environment,
    onOpenDialog,
}: HeaderProps): React.JSX.Element => (
    <header className={"page-header"}>
        <div className={"page-header-text"}>
            <h1 className={"page-title"}>{title}</h1>
            <p className={"page-subtitle"}>{subtitle}</p>
            <span className={`env-chip env-${environment}`}>{environment}</span>
        </div>
        <div className={"page-header-actions"}>
            <Button
                label={"Export snapshot"}
                variant={"secondary"}
                disabled={false}
                onClick={() => onOpenDialog("notes")}
            />
            <Button
                label={"Rollback"}
                variant={"danger"}
                disabled={environment === "staging"}
                onClick={() => onOpenDialog("confirm")}
            />
        </div>
    </header>
);

const SidebarNav = ({
    groups,
    activeId,
    onSelect,
}: SidebarNavProps): React.JSX.Element => (
    <nav className={"sidebar"}>
        {groups.map((group): React.JSX.Element => (
            <div
                className={"sidebar-group"}
                key={group.id}
            >
                <h3 className={"sidebar-group-label"}>{group.label}</h3>
                <ul className={"sidebar-items"}>
                    {group.items.map((item): React.JSX.Element => (
                        <li key={item.id}>
                            <button
                                type={"button"}
                                className={
                                    item.id === activeId
                                        ? "sidebar-item active"
                                        : "sidebar-item"
                                }
                                onClick={() => onSelect(item.id)}
                            >
                                <span>{item.label}</span>
                                {item.count > 0 && (
                                    <span className={"sidebar-count"}>
                                        {item.count}
                                    </span>
                                )}
                            </button>
                        </li>
                    ))}
                </ul>
            </div>
        ))}
    </nav>
);

const StatCardTile = ({ card }: StatCardTileProps): React.JSX.Element => (
    <article className={"stat-card"}>
        <span className={"stat-label"}>{card.label}</span>
        <span className={"stat-value"}>
            {card.value}
            <span className={"stat-unit"}>{card.unit}</span>
        </span>
        <div className={"stat-meta"}>
            <Badge
                label={`${trendSymbol(card.trend)} ${card.delta}${card.unit}`}
                tone={toneForTrend(card.trend)}
            />
            <span className={"stat-note"}>{card.note}</span>
        </div>
    </article>
);

const StatCardGrid = ({ cards }: StatCardGridProps): React.JSX.Element => (
    <div className={"stat-grid"}>
        {cards.map((card): React.JSX.Element => (
            <StatCardTile
                key={card.id}
                card={card}
            />
        ))}
    </div>
);

const CardTile = ({ card, onPromote }: CardTileProps): React.JSX.Element => (
    <article className={"card"}>
        <span className={"card-title"}>{card.title}</span>
        <span className={"card-assignee"}>{card.assignee}</span>
        <div className={"card-tags"}>
            {card.tags.map((tag): React.JSX.Element => (
                <span
                    className={"card-tag"}
                    key={tag}
                >
                    {tag}
                </span>
            ))}
        </div>
        <Badge
            label={`${card.points} pts`}
            tone={card.points > 5 ? "warning" : "neutral"}
        />
        <Button
            label={"Promote"}
            variant={"ghost"}
            disabled={false}
            onClick={() => onPromote(card.id)}
        />
    </article>
);

const Column = ({ column, onPromote }: ColumnProps): React.JSX.Element => {
    const count: number = column.cards.length;
    const tiles = column.cards.map((card): React.JSX.Element => (
        <CardTile
            key={card.id}
            card={card}
            onPromote={onPromote}
        />
    ));

    return (
        <section
            className={count > column.wipLimit ? "column over-limit" : "column"}
        >
            <header className={"column-head"}>
                <h2 className={"column-title"}>
                    {statusLabels[column.status]}
                </h2>
                <Badge
                    label={`${count}/${column.wipLimit}`}
                    tone={toneForLoad(count, column.wipLimit)}
                />
            </header>
            {count === 0 && (
                <p className={"column-empty"}>{"No cards here."}</p>
            )}
            <div className={"column-cards"}>{tiles}</div>
        </section>
    );
};

const AddCardForm = ({
    draft,
    onDraftChange,
    onSubmit,
}: AddCardFormProps): React.JSX.Element => (
    <form className={"add-form"}>
        <TextInput
            value={draft}
            placeholder={"Add a card"}
            onChange={onDraftChange}
        />
        <Button
            label={"Add card"}
            variant={"primary"}
            disabled={draft.trim().length === 0}
            onClick={onSubmit}
        />
    </form>
);

const KanbanBoard = ({
    columns,
    draft,
    onDraftChange,
    onAddCard,
    onPromote,
}: KanbanBoardProps): React.JSX.Element => (
    <section className={"kanban"}>
        <h2 className={"section-title"}>{"Delivery board"}</h2>
        <div className={"kanban-columns"}>
            {columns.map((column): React.JSX.Element => (
                <Column
                    key={column.status}
                    column={column}
                    onPromote={onPromote}
                />
            ))}
        </div>
        <AddCardForm
            draft={draft}
            onDraftChange={onDraftChange}
            onSubmit={onAddCard}
        />
    </section>
);

const DataTable = ({
    columns,
    rows,
    sortId,
    direction,
    onSort,
    page,
    pageCount,
    onPageChange,
}: DataTableProps): React.JSX.Element => {
    const pageSize: number = 5;
    const start: number = (page - 1) * pageSize;
    const visible = rows.slice(start, start + pageSize);

    return (
        <section className={"data-table"}>
            <h2 className={"section-title"}>{"Service health"}</h2>
            <table className={"table"}>
                <thead>
                    <tr>
                        {columns.map((column): React.JSX.Element => (
                            <th key={column.id}>
                                {column.sortable ? (
                                    <button
                                        type={"button"}
                                        className={"table-sort"}
                                        onClick={() => onSort(column.id)}
                                    >
                                        {column.header}
                                        {sortId === column.id && (
                                            <span className={"table-sort-mark"}>
                                                {direction === "ascending"
                                                    ? "↑"
                                                    : direction === "descending"
                                                      ? "↓"
                                                      : "◇"}
                                            </span>
                                        )}
                                    </button>
                                ) : (
                                    column.header
                                )}
                            </th>
                        ))}
                    </tr>
                </thead>
                <tbody>
                    {visible.map((row): React.JSX.Element => (
                        <tr key={row.id}>
                            <td className={"table-service"}>{row.service}</td>
                            <td>{regionLabels[row.region]}</td>
                            <td>
                                <Badge
                                    label={row.severity}
                                    tone={toneForSeverity(row.severity)}
                                />
                            </td>
                            <td>{formatLatency(row.latencyMs)}</td>
                            <td>{formatUptime(row.uptimePct)}</td>
                            <td>{row.owner}</td>
                            <td>{row.incidents}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
            <div className={"pager"}>
                <Button
                    label={"Prev"}
                    variant={"ghost"}
                    disabled={page === 1}
                    onClick={() => onPageChange(Math.max(1, page - 1))}
                />
                {pageWindow(page, pageCount).map(
                    (candidate): React.JSX.Element => (
                        <Button
                            key={candidate}
                            label={String(candidate)}
                            variant={candidate === page ? "primary" : "ghost"}
                            disabled={false}
                            onClick={() => onPageChange(candidate)}
                        />
                    ),
                )}
                <Button
                    label={"Next"}
                    variant={"ghost"}
                    disabled={page === pageCount}
                    onClick={() => onPageChange(Math.min(pageCount, page + 1))}
                />
            </div>
        </section>
    );
};

const ActivityFeed = ({ events }: ActivityFeedProps): React.JSX.Element => (
    <section className={"activity"}>
        <h2 className={"section-title"}>{"Activity"}</h2>
        <ul className={"activity-list"}>
            {events.map((event): React.JSX.Element => (
                <li
                    className={"activity-row"}
                    key={event.id}
                >
                    <Badge
                        label={activityLabels[event.kind]}
                        tone={
                            event.kind === "incident"
                                ? "critical"
                                : event.kind === "deploy"
                                  ? "positive"
                                  : "neutral"
                        }
                    />
                    <span className={"activity-text"}>
                        <strong>{event.actor}</strong>
                        {` ${event.target}`}
                    </span>
                    <span className={"activity-detail"}>{event.detail}</span>
                    <span className={"activity-age"}>
                        {formatAge(event.minutesAgo)}
                    </span>
                </li>
            ))}
        </ul>
    </section>
);

const NotificationPreferences = ({
    channels,
    onToggle,
    onThresholdChange,
}: NotificationPreferencesProps): React.JSX.Element => (
    <section className={"notif-prefs"}>
        <h2 className={"section-title"}>{"Notification preferences"}</h2>
        <ul className={"notif-list"}>
            {channels.map((channel): React.JSX.Element => (
                <li
                    className={"notif-row"}
                    key={channel.id}
                >
                    <div className={"notif-main"}>
                        <span className={"notif-label"}>{channel.label}</span>
                        <span className={"notif-description"}>
                            {channel.description}
                        </span>
                        <span className={"notif-hint"}>
                            {channelHints[channel.kind]}
                        </span>
                    </div>
                    <Toggle
                        label={channel.kind}
                        checked={channel.enabled}
                        onChange={(next) => onToggle(channel.id, next)}
                    />
                    <label className={"threshold"}>
                        <span>{"threshold"}</span>
                        <input
                            type={"number"}
                            min={0}
                            max={100}
                            value={channel.threshold}
                            disabled={!channel.enabled}
                            onChange={(event) =>
                                onThresholdChange(
                                    channel.id,
                                    Number(event.target.value),
                                )
                            }
                        />
                    </label>
                </li>
            ))}
        </ul>
        <p className={"notif-summary"}>
            {channels.filter((channel) => channel.enabled).length}
            {" of "}
            {channels.length}
            {" channels enabled."}
        </p>
    </section>
);

const SettingsPanel = ({
    rows,
    onToggle,
    onChoice,
}: SettingsPanelProps): React.JSX.Element => (
    <section className={"settings"}>
        <h2 className={"section-title"}>{"Workspace settings"}</h2>
        <div className={"settings-rows"}>
            {rows.map((row): React.JSX.Element => {
                if (row.kind === "toggle") {
                    return (
                        <div
                            className={"settings-row"}
                            key={row.id}
                        >
                            <span className={"settings-label"}>
                                {row.label}
                            </span>
                            <Toggle
                                label={row.id}
                                checked={row.enabled}
                                onChange={(next) => onToggle(row.id, next)}
                            />
                        </div>
                    );
                }
                return (
                    <div
                        className={"settings-row"}
                        key={row.id}
                    >
                        <span className={"settings-label"}>{row.label}</span>
                        <div className={"settings-choices"}>
                            {row.options.map((option): React.JSX.Element => (
                                <Button
                                    key={option}
                                    label={option}
                                    variant={
                                        option === row.value
                                            ? "primary"
                                            : "secondary"
                                    }
                                    disabled={false}
                                    onClick={() => onChoice(row.id, option)}
                                />
                            ))}
                        </div>
                    </div>
                );
            })}
        </div>
    </section>
);

const Timeline = ({ milestones }: TimelineProps): React.JSX.Element => (
    <section className={"timeline"}>
        <h2 className={"section-title"}>{"Release timeline"}</h2>
        <ol className={"timeline-list"}>
            {milestones.map((milestone): React.JSX.Element => (
                <li
                    className={`timeline-item timeline-${milestone.status}`}
                    key={milestone.id}
                >
                    <div className={"timeline-head"}>
                        <span className={"timeline-label"}>
                            {milestone.label}
                        </span>
                        <Badge
                            label={milestone.status}
                            tone={
                                milestone.status === "shipped"
                                    ? "positive"
                                    : milestone.status === "active"
                                      ? "warning"
                                      : "neutral"
                            }
                        />
                    </div>
                    <div className={"timeline-body"}>
                        <span className={"timeline-window"}>
                            {milestone.window}
                        </span>
                        <span className={"timeline-owner"}>
                            {milestone.owner}
                        </span>
                        <span className={"timeline-progress"}>
                            {milestone.progress}
                            {"%"}
                        </span>
                    </div>
                    <div className={"timeline-bar"}>
                        <div
                            className={"timeline-fill"}
                            style={{
                                width: `${milestone.progress}%`,
                            }}
                        />
                    </div>
                </li>
            ))}
        </ol>
    </section>
);

const TabBar = ({
    active,
    labels,
    onSelect,
}: TabBarProps): React.JSX.Element => (
    <section className={"tabs"}>
        <div className={"tab-bar"}>
            {(["metrics", "logs", "traces"] as Array<TabId>).map(
                (id): React.JSX.Element => (
                    <button
                        type={"button"}
                        className={id === active ? "tab active" : "tab"}
                        key={id}
                        onClick={() => onSelect(id)}
                    >
                        {labels[id]}
                    </button>
                ),
            )}
        </div>
        <div className={"tab-panel"}>
            {active === "metrics" && (
                <p className={"tab-text"}>
                    {"Latency histograms refresh every sixty seconds."}
                </p>
            )}
            {active === "logs" && (
                <p className={"tab-text"}>
                    {"Structured logs stream from the edge collectors."}
                </p>
            )}
            {active === "traces" && (
                <p className={"tab-text"}>
                    {"Trace sampling is capped at five percent per route."}
                </p>
            )}
        </div>
    </section>
);

const ConfirmDialog = ({
    open,
    title,
    body,
    onCancel,
    onConfirm,
}: ConfirmDialogProps): React.JSX.Element => {
    if (!open) return <div className={"dialog-slot"} />;

    return (
        <div className={"dialog-backdrop"}>
            <div
                className={"dialog"}
                role={"dialog"}
            >
                <h3 className={"dialog-title"}>{title}</h3>
                <p className={"dialog-body"}>{body}</p>
                <div className={"dialog-actions"}>
                    <Button
                        label={"Cancel"}
                        variant={"secondary"}
                        disabled={false}
                        onClick={onCancel}
                    />
                    <Button
                        label={"Confirm rollback"}
                        variant={"danger"}
                        disabled={false}
                        onClick={onConfirm}
                    />
                </div>
            </div>
        </div>
    );
};

const NotesDialog = ({
    open,
    body,
    onClose,
}: NotesDialogProps): React.JSX.Element => (
    <div className={"dialog-slot"}>
        {open && (
            <div className={"dialog-backdrop"}>
                <div
                    className={"dialog"}
                    role={"dialog"}
                >
                    <h3 className={"dialog-title"}>{"Snapshot notes"}</h3>
                    <p className={"dialog-body"}>{body}</p>
                    <div className={"dialog-actions"}>
                        <Button
                            label={"Close"}
                            variant={"ghost"}
                            disabled={false}
                            onClick={onClose}
                        />
                    </div>
                </div>
            </div>
        )}
    </div>
);

const TeamRoster = ({ members }: TeamRosterProps): React.JSX.Element => (
    <section className={"roster"}>
        <h2 className={"section-title"}>{"Team roster"}</h2>
        <ul className={"roster-list"}>
            {members.map((member): React.JSX.Element => (
                <li
                    className={"roster-row"}
                    key={member.id}
                >
                    <span className={"roster-name"}>{member.name}</span>
                    <span className={"roster-role"}>{member.role}</span>
                    <span className={"roster-region"}>
                        {regionLabels[member.region]}
                    </span>
                    <Badge
                        label={`load ${member.load}`}
                        tone={
                            member.load > 90
                                ? "critical"
                                : member.load < 40
                                  ? "neutral"
                                  : "positive"
                        }
                    />
                </li>
            ))}
        </ul>
        <p className={"roster-note"}>{capacityNote}</p>
    </section>
);

const Footer = ({ note }: FooterProps): React.JSX.Element => (
    <footer className={"page-footer"}>
        <span className={"page-footer-note"}>{note}</span>
    </footer>
);

const seedColumns: Array<ColumnData> = [
    {
        status: "backlog",
        wipLimit: 5,
        cards: [
            {
                id: "c-1",
                title: "Spike a streaming diff format",
                points: 3,
                assignee: "priya",
                tags: ["spike", "codec"],
            },
            {
                id: "c-2",
                title: "Audit the fixture corpus",
                points: 2,
                assignee: "marc",
                tags: ["quality"],
            },
            {
                id: "c-3",
                title: "Draft the reader benchmarks",
                points: 5,
                assignee: "lena",
                tags: ["bench", "reader"],
            },
            {
                id: "c-4",
                title: "Evaluate string interning",
                points: 8,
                assignee: "tomas",
                tags: ["perf", "reader"],
            },
        ],
    },
    {
        status: "todo",
        wipLimit: 4,
        cards: [
            {
                id: "c-5",
                title: "Pick a vendor for tracing",
                points: 5,
                assignee: "sana",
                tags: ["infra"],
            },
            {
                id: "c-6",
                title: "Wire the snapshot importer",
                points: 3,
                assignee: "priya",
                tags: ["codec", "import"],
            },
            {
                id: "c-7",
                title: "Document the union reader",
                points: 2,
                assignee: "lena",
                tags: ["docs"],
            },
        ],
    },
    {
        status: "doing",
        wipLimit: 3,
        cards: [
            {
                id: "c-8",
                title: "Wire the bench harness",
                points: 8,
                assignee: "marc",
                tags: ["bench"],
            },
            {
                id: "c-9",
                title: "Stabilize the snapshot tests",
                points: 5,
                assignee: "tomas",
                tags: ["quality"],
            },
        ],
    },
    {
        status: "review",
        wipLimit: 2,
        cards: [
            {
                id: "c-10",
                title: "Migrate the table fixtures",
                points: 3,
                assignee: "sana",
                tags: ["migration"],
            },
        ],
    },
    {
        status: "done",
        wipLimit: 6,
        cards: [
            {
                id: "c-11",
                title: "Ship the array reader",
                points: 5,
                assignee: "priya",
                tags: ["reader"],
            },
            {
                id: "c-12",
                title: "Land the object reader",
                points: 5,
                assignee: "marc",
                tags: ["reader"],
            },
            {
                id: "c-13",
                title: "Retire the legacy parser",
                points: 2,
                assignee: "lena",
                tags: ["cleanup"],
            },
        ],
    },
];

const navGroups: Array<NavGroup> = [
    {
        id: "group-overview",
        label: "Overview",
        items: [
            { id: "nav-metrics", label: "Metrics", count: 0 },
            { id: "nav-boards", label: "Boards", count: 3 },
            { id: "nav-timeline", label: "Timeline", count: 0 },
        ],
    },
    {
        id: "group-reliability",
        label: "Reliability",
        items: [
            { id: "nav-services", label: "Services", count: 18 },
            { id: "nav-incidents", label: "Incidents", count: 2 },
            { id: "nav-slo", label: "SLO budgets", count: 5 },
        ],
    },
    {
        id: "group-team",
        label: "Team",
        items: [
            { id: "nav-roster", label: "Roster", count: 0 },
            { id: "nav-queues", label: "Review queues", count: 7 },
            { id: "nav-oncall", label: "On-call", count: 1 },
        ],
    },
    {
        id: "group-platform",
        label: "Platform",
        items: [
            { id: "nav-settings", label: "Settings", count: 0 },
            { id: "nav-channels", label: "Channels", count: 4 },
        ],
    },
];

const statCards: Array<StatCard> = [
    {
        id: "stat-throughput",
        label: "Throughput",
        value: 18_420,
        unit: "rps",
        trend: "up",
        delta: 6.2,
        note: "Peak window 14:00 UTC",
    },
    {
        id: "stat-latency",
        label: "p99 latency",
        value: 212,
        unit: "ms",
        trend: "down",
        delta: 4.8,
        note: "Improved after cache fix",
    },
    {
        id: "stat-error-rate",
        label: "Error rate",
        value: 0.42,
        unit: "%",
        trend: "flat",
        delta: 0.0,
        note: "Within the SLO budget",
    },
    {
        id: "stat-deploys",
        label: "Deploys today",
        value: 9,
        unit: "",
        trend: "up",
        delta: 3.0,
        note: "Two trains still pending",
    },
];

const tableColumns: Array<TableColumn> = [
    { id: "service", header: "Service", sortable: true },
    { id: "region", header: "Region", sortable: true },
    { id: "severity", header: "Severity", sortable: true },
    { id: "latency", header: "p99", sortable: false },
    { id: "uptime", header: "Uptime", sortable: false },
    { id: "owner", header: "Owner", sortable: true },
    { id: "incidents", header: "Incidents", sortable: true },
];

const tableRows: Array<TableRow> = [
    {
        id: "row-1",
        service: "edge-router",
        region: "us",
        severity: "info",
        latencyMs: 88,
        uptimePct: 99.98,
        owner: "platform",
        incidents: 0,
    },
    {
        id: "row-2",
        service: "auth-gateway",
        region: "eu",
        severity: "major",
        latencyMs: 340,
        uptimePct: 99.51,
        owner: "identity",
        incidents: 1,
    },
    {
        id: "row-3",
        service: "search-index",
        region: "apac",
        severity: "severe",
        latencyMs: 512,
        uptimePct: 98.72,
        owner: "discovery",
        incidents: 2,
    },
    {
        id: "row-4",
        service: "billing-ledger",
        region: "us",
        severity: "minor",
        latencyMs: 156,
        uptimePct: 99.9,
        owner: "payments",
        incidents: 1,
    },
    {
        id: "row-5",
        service: "webhook-relay",
        region: "eu",
        severity: "info",
        latencyMs: 74,
        uptimePct: 99.99,
        owner: "platform",
        incidents: 0,
    },
    {
        id: "row-6",
        service: "media-encoder",
        region: "us",
        severity: "minor",
        latencyMs: 221,
        uptimePct: 99.83,
        owner: "media",
        incidents: 0,
    },
    {
        id: "row-7",
        service: "presence-hub",
        region: "apac",
        severity: "info",
        latencyMs: 95,
        uptimePct: 99.97,
        owner: "realtime",
        incidents: 0,
    },
    {
        id: "row-8",
        service: "audit-sink",
        region: "eu",
        severity: "info",
        latencyMs: 62,
        uptimePct: 99.99,
        owner: "compliance",
        incidents: 0,
    },
    {
        id: "row-9",
        service: "graph-resolver",
        region: "us",
        severity: "minor",
        latencyMs: 187,
        uptimePct: 99.86,
        owner: "api",
        incidents: 1,
    },
    {
        id: "row-10",
        service: "notify-sender",
        region: "apac",
        severity: "major",
        latencyMs: 298,
        uptimePct: 99.42,
        owner: "messaging",
        incidents: 1,
    },
];

const activityEvents: Array<ActivityEvent> = [
    {
        id: "ev-1",
        kind: "deploy",
        actor: "priya",
        target: "edge-router@2.14.0",
        minutesAgo: 12,
        detail: "Canary at 5% traffic",
    },
    {
        id: "ev-2",
        kind: "incident",
        actor: "sana",
        target: "search-index",
        minutesAgo: 41,
        detail: "Reindex queue backpressure",
    },
    {
        id: "ev-3",
        kind: "review",
        actor: "lena",
        target: "codec#482",
        minutesAgo: 66,
        detail: "Union reader refactor",
    },
    {
        id: "ev-4",
        kind: "merge",
        actor: "marc",
        target: "bench#91",
        minutesAgo: 120,
        detail: "Large fixture wired",
    },
    {
        id: "ev-5",
        kind: "comment",
        actor: "tomas",
        target: "codec#479",
        minutesAgo: 210,
        detail: "Requested string stats",
    },
    {
        id: "ev-6",
        kind: "deploy",
        actor: "priya",
        target: "auth-gateway@5.2.1",
        minutesAgo: 300,
        detail: "Full rollout complete",
    },
    {
        id: "ev-7",
        kind: "incident",
        actor: "marc",
        target: "auth-gateway",
        minutesAgo: 640,
        detail: "Mitigated by rollback",
    },
    {
        id: "ev-8",
        kind: "review",
        actor: "sana",
        target: "table#17",
        minutesAgo: 980,
        detail: "Pagination behavior",
    },
    {
        id: "ev-9",
        kind: "merge",
        actor: "lena",
        target: "reader#305",
        minutesAgo: 1500,
        detail: "Object reader landed",
    },
];

const notificationChannels: Array<NotificationChannel> = [
    {
        id: "chan-email",
        kind: "email",
        label: "Daily digest",
        description:
            "A morning summary of board movement, incidents and review load.",
        enabled: true,
        threshold: 20,
    },
    {
        id: "chan-slack",
        kind: "slack",
        label: "Release channel",
        description:
            "Deploy and merge events posted to the release train channel.",
        enabled: true,
        threshold: 50,
    },
    {
        id: "chan-sms",
        kind: "sms",
        label: "Pager escalations",
        description:
            "Text messages for severity major and above, 24 by 7 rotation.",
        enabled: false,
        threshold: 80,
    },
    {
        id: "chan-webhook",
        kind: "webhook",
        label: "Raw event feed",
        description:
            "Undecorated JSON payloads pushed to the analytics pipeline.",
        enabled: true,
        threshold: 10,
    },
];

const settingRows: Array<SettingRow> = [
    {
        id: "set-compact",
        kind: "toggle",
        label: "Compact tables",
        enabled: true,
    },
    {
        id: "set-animations",
        kind: "toggle",
        label: "Motion effects",
        enabled: false,
    },
    {
        id: "set-sound",
        kind: "toggle",
        label: "Alert sounds",
        enabled: true,
    },
    {
        id: "set-density",
        kind: "choice",
        label: "Row density",
        value: "comfortable",
        options: ["compact", "comfortable", "spacious"],
    },
    {
        id: "set-refresh",
        kind: "choice",
        label: "Refresh cadence",
        value: "60s",
        options: ["15s", "60s", "5m"],
    },
    {
        id: "set-theme",
        kind: "choice",
        label: "Color theme",
        value: "system",
        options: ["light", "dark", "system"],
    },
];

const timelineMilestones: Array<Milestone> = [
    {
        id: "ms-1",
        label: "Reader hardening",
        window: "Week 1 to Week 2",
        status: "shipped",
        progress: 100,
        owner: "priya",
    },
    {
        id: "ms-2",
        label: "Bench rewiring",
        window: "Week 3",
        status: "active",
        progress: 62,
        owner: "marc",
    },
    {
        id: "ms-3",
        label: "Corpus refresh",
        window: "Week 4",
        status: "active",
        progress: 28,
        owner: "lena",
    },
    {
        id: "ms-4",
        label: "Snapshot v2 format",
        window: "Week 5 to Week 6",
        status: "planned",
        progress: 0,
        owner: "tomas",
    },
    {
        id: "ms-5",
        label: "Docs and release",
        window: "Week 7",
        status: "planned",
        progress: 0,
        owner: "sana",
    },
];

const teamMembers: Array<TeamMember> = [
    { id: "m-1", name: "Priya", role: "Codec core", region: "eu", load: 72 },
    { id: "m-2", name: "Marc", role: "Benchmarks", region: "us", load: 88 },
    { id: "m-3", name: "Lena", role: "Fixtures", region: "eu", load: 35 },
    { id: "m-4", name: "Tomas", role: "Readers", region: "apac", load: 94 },
    { id: "m-5", name: "Sana", role: "Platform", region: "apac", load: 51 },
];

const Page = (): React.JSX.Element => {
    const [columns, setColumns] =
        React.useState<Array<ColumnData>>(seedColumns);
    const [draft, setDraft] = React.useState<string>("");
    const [activeNav, setActiveNav] = React.useState<string>("nav-metrics");
    const [dialog, setDialog] = React.useState<DialogKind>("none");
    const [tab, setTab] = React.useState<TabId>("metrics");
    const [sortId, setSortId] = React.useState<string>("service");
    const [direction, setDirection] =
        React.useState<SortDirection>("ascending");
    const [page, setPage] = React.useState<number>(1);
    const [channels, setChannels] =
        React.useState<Array<NotificationChannel>>(notificationChannels);
    const [settings, setSettings] =
        React.useState<Array<SettingRow>>(settingRows);
    const [confirmNote, setConfirmNote] = React.useState<string>("");

    const handlePromote = React.useCallback(
        (id: string): void => {
            setColumns((current) => {
                const index: number = current.findIndex((column) =>
                    column.cards.some((entry) => entry.id === id),
                );
                if (index === -1) return current;
                const card = current[index].cards.find(
                    (entry) => entry.id === id,
                );
                if (!card) return current;
                const nextIndex: number = Math.min(
                    index + 1,
                    columnOrder.length - 1,
                );

                return current.map((column, position): ColumnData => {
                    const kept = column.cards.filter(
                        (entry) => entry.id !== id,
                    );
                    if (position === nextIndex)
                        return { ...column, cards: [...kept, card] };
                    return { ...column, cards: kept };
                });
            });
        },
        [setColumns],
    );

    const handleAddCard = React.useCallback((): void => {
        const title: string = draft.trim();
        if (title.length === 0) return void 0;
        const card: Card = {
            id: `c-new-${title.length}-${columns.length}`,
            title,
            points: 3,
            assignee: "unassigned",
            tags: ["new"],
        };
        setColumns((current) =>
            current.map((column): ColumnData =>
                column.status === "backlog"
                    ? { ...column, cards: [...column.cards, card] }
                    : column,
            ),
        );
        setDraft("");
    }, [draft, columns, setColumns, setDraft]);

    const handleDraftChange = React.useCallback(
        (next: string): void => setDraft(next),
        [setDraft],
    );

    const handleNavSelect = React.useCallback(
        (id: string): void => setActiveNav(id),
        [setActiveNav],
    );

    const handleOpenDialog = React.useCallback(
        (kind: DialogKind): void => {
            setDialog(kind);
            if (kind === "confirm")
                setConfirmNote(
                    "Rolling back reverts the most recent train across all regions.",
                );
        },
        [setDialog, setConfirmNote],
    );

    const handleCloseDialog = React.useCallback(
        (): void => setDialog("none"),
        [setDialog],
    );

    const handleConfirm = React.useCallback((): void => {
        setDialog("none");
        setConfirmNote("");
    }, [setDialog, setConfirmNote]);

    const handleSort = React.useCallback(
        (id: string): void => {
            if (id === sortId) {
                setDirection(sortDirectionAfter(direction));
                return;
            }
            setSortId(id);
            setDirection("ascending");
        },
        [sortId, direction, setSortId, setDirection],
    );

    const sortedRows = React.useMemo((): Array<TableRow> => {
        if (direction === "none") return tableRows;
        const factor: number = direction === "ascending" ? 1 : -1;
        const rows: Array<TableRow> = tableRows.slice();
        rows.sort((left: TableRow, right: TableRow) => {
            if (sortId === "latency")
                return factor * (left.latencyMs - right.latencyMs);
            if (sortId === "incidents")
                return factor * (left.incidents - right.incidents);
            return factor * left.id.localeCompare(right.id);
        });
        return rows;
    }, [sortId, direction]);

    const pageCount: number = Math.ceil(tableRows.length / 5);

    const handlePageChange = React.useCallback(
        (next: number): void => setPage(next),
        [setPage],
    );

    const handleChannelToggle = React.useCallback(
        (id: string, next: boolean): void => {
            setChannels((current) =>
                current.map((channel): NotificationChannel =>
                    channel.id === id ? { ...channel, enabled: next } : channel,
                ),
            );
        },
        [setChannels],
    );

    const handleThresholdChange = React.useCallback(
        (id: string, next: number): void => {
            setChannels((current) =>
                current.map((channel): NotificationChannel =>
                    channel.id === id
                        ? { ...channel, threshold: next }
                        : channel,
                ),
            );
        },
        [setChannels],
    );

    const handleSettingToggle = React.useCallback(
        (id: string, next: boolean): void => {
            setSettings((current) =>
                current.map((row): SettingRow =>
                    row.id === id && row.kind === "toggle"
                        ? { ...row, enabled: next }
                        : row,
                ),
            );
        },
        [setSettings],
    );

    const handleSettingChoice = React.useCallback(
        (id: string, next: string): void => {
            setSettings((current) =>
                current.map((row): SettingRow =>
                    row.id === id && row.kind === "choice"
                        ? { ...row, value: next }
                        : row,
                ),
            );
        },
        [setSettings],
    );

    const totalPoints = React.useMemo(
        (): number =>
            columns.reduce(
                (sum, column) =>
                    sum +
                    column.cards.reduce(
                        (inner, card) => inner + card.points,
                        0,
                    ),
                0,
            ),
        [columns],
    );

    const openIncidents = React.useMemo(
        (): number => tableRows.reduce((sum, row) => sum + row.incidents, 0),
        [],
    );

    return (
        <div className={"page"}>
            <Header
                title={"Codec Control"}
                subtitle={pageIntro}
                environment={"production"}
                onOpenDialog={handleOpenDialog}
            />
            <div className={"page-body"}>
                <SidebarNav
                    groups={navGroups}
                    activeId={activeNav}
                    onSelect={handleNavSelect}
                />
                <main className={"page-main"}>
                    <StatCardGrid cards={statCards} />
                    <KanbanBoard
                        columns={columns}
                        draft={draft}
                        onDraftChange={handleDraftChange}
                        onAddCard={handleAddCard}
                        onPromote={handlePromote}
                    />
                    <DataTable
                        columns={tableColumns}
                        rows={sortedRows}
                        sortId={sortId}
                        direction={direction}
                        onSort={handleSort}
                        page={page}
                        pageCount={pageCount}
                        onPageChange={handlePageChange}
                    />
                    <ActivityFeed events={activityEvents} />
                    <div className={"page-columns"}>
                        <NotificationPreferences
                            channels={channels}
                            onToggle={handleChannelToggle}
                            onThresholdChange={handleThresholdChange}
                        />
                        <SettingsPanel
                            rows={settings}
                            onToggle={handleSettingToggle}
                            onChoice={handleSettingChoice}
                        />
                    </div>
                    <TabBar
                        active={tab}
                        labels={tabLabels}
                        onSelect={setTab}
                    />
                    <Timeline milestones={timelineMilestones} />{" "}
                    <TeamRoster members={teamMembers} />
                    <div className={"insights"}>
                        <p className={"insight-line"}>
                            {"Delivery board carries "}
                            {totalPoints}
                            {" points across "}
                            {columns.length}
                            {" columns."}
                        </p>
                        <p className={"insight-line"}>{incidentSummary}</p>
                    </div>
                    {openIncidents > 2 && (
                        <div className={"page-banner"}>
                            <span>{"Incident budget is under pressure."}</span>
                            <Button
                                label={"Review incidents"}
                                variant={"secondary"}
                                disabled={false}
                                onClick={() => setActiveNav("nav-incidents")}
                            />
                        </div>
                    )}
                </main>
            </div>
            <ConfirmDialog
                open={dialog === "confirm"}
                title={"Rollback the last train?"}
                body={confirmNote}
                onCancel={handleCloseDialog}
                onConfirm={handleConfirm}
            />
            <NotesDialog
                open={dialog === "notes"}
                body={pageIntro}
                onClose={handleCloseDialog}
            />
            <Footer note={"Built for the codec benchmark suite"} />
        </div>
    );
};

export default Page;
