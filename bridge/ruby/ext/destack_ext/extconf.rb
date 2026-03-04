require "mkmf"

if RUBY_PLATFORM =~ /linux|bsd/
  have_library("dl")
end

create_makefile("destack_ext")
