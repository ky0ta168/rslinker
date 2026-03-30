# i686-w64-mingw32-gcc のパスを通す
export PATH := "/c/msys64/mingw32/bin:" + env_var("PATH")

# 利用可能なレシピ一覧を表示
default:
    @just --list

# examples/hello/hello.c をコンパイルして rslinker でリンクし hello.exe を生成する
hello:
    i686-w64-mingw32-gcc -c examples/hello/hello.c -o examples/hello/hello.obj
    cargo run -- examples/hello/hello.obj -out examples/hello/hello.exe

# examples/hello_dll/ の DLL とバイナリを rslinker でリンクし hello_dll.exe を生成する
hello_dll:
    i686-w64-mingw32-gcc -shared -o examples/hello_dll/mylib.dll examples/hello_dll/mylib.c -luser32
    i686-w64-mingw32-gcc -c examples/hello_dll/hello_dll.c -o examples/hello_dll/hello_dll.obj
    cargo run -- examples/hello_dll/hello_dll.obj -dll examples/hello_dll/mylib.dll -out examples/hello_dll/hello_dll.exe

# 生成ファイルを削除する
clean:
    rm -f examples/hello/hello.obj examples/hello/hello.exe
    rm -f examples/hello_dll/hello_dll.obj examples/hello_dll/hello_dll.exe examples/hello_dll/mylib.dll
