RSLINKER = cargo run --
GCC      = i686-w64-mingw32-gcc
PATH    := /c/msys64/mingw32/bin:$(PATH)

all: examples

examples: examples/hello examples/hello_dll

examples/hello:
	$(GCC) -c examples/hello/hello.c -o examples/hello/hello.obj
	$(RSLINKER) examples/hello/hello.obj -out examples/hello/hello.exe

examples/hello_dll:
	$(GCC) -shared -o examples/hello_dll/mylib.dll examples/hello_dll/mylib.c -luser32
	$(GCC) -c examples/hello_dll/hello_dll.c -o examples/hello_dll/hello_dll.obj
	$(RSLINKER) examples/hello_dll/hello_dll.obj -dll examples/hello_dll/mylib.dll -out examples/hello_dll/hello_dll.exe

clean:
	rm -f examples/hello/hello.obj examples/hello/hello.exe
	rm -f examples/hello_dll/hello_dll.obj examples/hello_dll/hello_dll.exe examples/hello_dll/mylib.dll

.PHONY: all examples examples/hello examples/hello_dll clean
