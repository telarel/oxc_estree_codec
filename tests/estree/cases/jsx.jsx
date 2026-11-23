const props = { className: "card", id: "main" };
const title = "Hello";
const element = (
    <section {...props} className="card">
        <h1 id="title">{title}</h1>
        <p>Text and {title} and {1 + 1}</p>
        <input disabled={false} />
    </section>
);
export default element;
