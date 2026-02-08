TS ?= tree-sitter

all install uninstall clean:
	$(MAKE) -C typescript $@
	$(MAKE) -C tsx $@
	$(MAKE) -C destack $@

test:
	$(TS) test
	$(TS) parse examples/* --quiet --time

.PHONY: all install uninstall clean test update
