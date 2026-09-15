@dec
class C {
    @x m() {}
    @y p = 1;
    constructor(
        private a: Foo,
        public b = 1,
        readonly c,
        protected d?,
    ) {}
}
