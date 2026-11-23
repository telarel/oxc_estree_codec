interface Props {
    readonly name: string;
    count?: number;
}

function Greeting({ name, count = 1 }: Props): JSX.Element {
    return (
        <div className="greeting" data-count={count}>
            Hello, {name}!
        </div>
    );
}

const props = { name: "Ada", count: 2 } satisfies Props;
const element = <Greeting {...props} />;
export default element;
