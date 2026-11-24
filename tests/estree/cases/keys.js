const key = "dynamic";

const object = {
    plain: 1,
    "string": 2,
    3: 3,
    [key]: 4,
    ["computed string"]: 5,
    method() {},
    "quoted method"() {},
    6() {},
    [key]() {},
};

const { plain, "string": renamed, 3: numeric, [key]: computed } = object;

({ plain, "string": renamed, 3: numeric, [key]: computed } = object);

class Example {
    method() {}
    "quoted method"() {}
    7() {}
    [key]() {}
    get value() {
        return object.plain;
    }
    set value(next) {
        object.plain = next;
    }
    static staticMethod() {}
    static [key]() {}
}

export { Example, object };
