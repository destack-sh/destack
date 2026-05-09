let _this = this;
const Normal = {
    foo: (function () {
        let _ref = babelHelpers.asyncToGenerator(function* () {
            console.log(log);
        });
        return function foo() {
            return _ref.apply(this, arguments);
        };
    })(),
};
const StringLiteralKey = {
    ["bar"]: (function () {
        let _ref2 = babelHelpers.asyncToGenerator(function* () {});
        return function bar() {
            return _ref2.apply(this, arguments);
        };
    })(),
};
const EmptyStringLiteralKey = {
    [""]: (function () {
        let _ref3 = babelHelpers.asyncToGenerator(function* () {
            console.log(_this);
        });
        return function _() {
            return _ref3.apply(this, arguments);
        };
    })(),
};
const InvalidStringLiteralKey = {
    ["#"]: (function () {
        let _ref4 = babelHelpers.asyncToGenerator(function* () {});
        return function _() {
            return _ref4.apply(this, arguments);
        };
    })(),
    ["this"]: (function () {
        let _ref5 = babelHelpers.asyncToGenerator(function* () {});
        return function _this() {
            return _ref5.apply(this, arguments);
        };
    })(),
    ["#default"]: (function () {
        let _ref6 = babelHelpers.asyncToGenerator(function* () {});
        return function _default() {
            return _ref6.apply(this, arguments);
        };
    })(),
    ["O X C"]: (function () {
        let _ref7 = babelHelpers.asyncToGenerator(function* () {});
        return function O_X_C() {
            return _ref7.apply(this, arguments);
        };
    })(),
};
