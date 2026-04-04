var o = class {
  #a;
  foo = class {
    #s;
    #e;
    #f;
  };
  get #o() {
  }
  set #o(e) {
  }
};
var a = class {
  #a;
  foo = class {
    #s;
    #e;
    #f;
  };
  get #o() {
  }
  set #o(e) {
  }
};
var f = [o, a];
export {
  f as privateIdentifierShapes
};
