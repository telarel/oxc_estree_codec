import * as React from "react";

type Card = { id: string; title: string; points: number };

type ColumnStatus = "todo" | "doing" | "done";

type BadgeTone = "neutral" | "positive" | "warning" | "critical";

type ButtonVariant = "primary" | "secondary" | "ghost";

type ColumnData = {
    status: ColumnStatus;
    wipLimit: number;
    cards: Array<Card>;
};

type BadgeProps = { label: string; tone: BadgeTone };

type ButtonProps = {
    label: string;
    variant: ButtonVariant;
    onClick: () => void;
};

type CardTileProps = { card: Card; onPromote: (id: string) => void };

type ColumnProps = { column: ColumnData; onPromote: (id: string) => void };

type AddCardFormProps = {
    draft: string;
    onDraftChange: (value: string) => void;
    onSubmit: () => void;
};

const columnOrder: Array<ColumnStatus> = ["todo", "doing", "done"];

const statusLabels: Record<ColumnStatus, string> = {
    todo: "To do",
    doing: "Doing",
    done: "Done",
};

const toneForLoad = (count: number, wipLimit: number): BadgeTone => {
    if (count >= wipLimit) return "critical";
    if (count === wipLimit - 1) return "warning";
    return "neutral";
};

const Badge = ({ label, tone }: BadgeProps): React.JSX.Element => (
    <span className={`badge badge-${tone}`}>{label}</span>
);

const Button = ({
    label,
    variant,
    onClick,
}: ButtonProps): React.JSX.Element => (
    <button
        type={"button"}
        className={`btn btn-${variant}`}
        onClick={onClick}
    >
        {label}
    </button>
);

const CardTile = ({ card, onPromote }: CardTileProps): React.JSX.Element => (
    <article className={"card"}>
        <span className={"card-title"}>{card.title}</span>
        <Badge
            label={`${card.points} pts`}
            tone={card.points > 5 ? "warning" : "neutral"}
        />
        <Button
            label={"Promote"}
            variant={"ghost"}
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
        <input
            type={"text"}
            value={draft}
            placeholder={"Add a card"}
            onChange={(event) => onDraftChange(event.target.value)}
        />
        <Button
            label={"Add card"}
            variant={"primary"}
            onClick={onSubmit}
        />
    </form>
);

const seedColumns: Array<ColumnData> = [
    {
        status: "todo",
        wipLimit: 4,
        cards: [
            { id: "c-1", title: "Draft the RFC", points: 2 },
            { id: "c-2", title: "Pick a vendor", points: 5 },
        ],
    },
    {
        status: "doing",
        wipLimit: 2,
        cards: [{ id: "c-3", title: "Wire the bench", points: 8 }],
    },
    {
        status: "done",
        wipLimit: 6,
        cards: [{ id: "c-4", title: "Migrate fixtures", points: 3 }],
    },
];

const Page = (): React.JSX.Element => {
    const [columns, setColumns] =
        React.useState<Array<ColumnData>>(seedColumns);
    const [draft, setDraft] = React.useState<string>("");

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

    const counts = React.useMemo(
        (): Record<ColumnStatus, number> => ({
            todo: columns[0].cards.length,
            doing: columns[1].cards.length,
            done: columns[2].cards.length,
        }),
        [columns],
    );

    const total = React.useMemo(
        (): number =>
            columns.reduce((sum, column) => sum + column.cards.length, 0),
        [columns],
    );

    const handleDraftChange = React.useCallback(
        (value: string): void => setDraft(value),
        [setDraft],
    );

    const handleSubmit = React.useCallback((): void => {
        const title: string = draft.trim();
        if (title.length === 0) return void 0;
        const card: Card = {
            id: `c-${total + 1}-${title.length}`,
            title,
            points: 3,
        };
        setColumns((current) =>
            current.map((column): ColumnData =>
                column.status === "todo"
                    ? { ...column, cards: [...column.cards, card] }
                    : column,
            ),
        );
        setDraft("");
    }, [draft, total, setColumns]);

    return (
        <div className={"page"}>
            <header className={"page-header"}>
                <h1 className={"page-title"}>{"Codec Board"}</h1>
                <p className={"page-subtitle"}>
                    {"A kanban page rendered by the fixture"}
                </p>
            </header>
            {columns.map((column): React.JSX.Element => (
                <Column
                    key={column.status}
                    column={column}
                    onPromote={handlePromote}
                />
            ))}
            <AddCardForm
                draft={draft}
                onDraftChange={handleDraftChange}
                onSubmit={handleSubmit}
            />
            {total > 0 && (
                <div className={"summary"}>
                    {columnOrder.map((status): React.JSX.Element => (
                        <Badge
                            key={status}
                            label={`${statusLabels[status]}: ${counts[status]}`}
                            tone={counts[status] === 0 ? "neutral" : "positive"}
                        />
                    ))}
                </div>
            )}
            <footer className={"page-footer"}>
                <span className={"page-footer-note"}>
                    {"Built for the codec benchmark"}
                </span>
            </footer>
        </div>
    );
};

export default Page;
