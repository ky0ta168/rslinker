# i686-w64-mingw32-gcc のパスを通す
export PATH := "/c/msys64/mingw32/bin:" + env_var("PATH")

# 利用可能なレシピ一覧を表示
default:
    @just --list

# examples/multi/ の 3 つの .obj と mathlib.dll を rslinker でまとめてリンクする
multi:
    i686-w64-mingw32-gcc -shared -o examples/multi/mathlib.dll examples/multi/mathlib.c
    i686-w64-mingw32-gcc -c examples/multi/math.c    -o examples/multi/math.obj
    i686-w64-mingw32-gcc -c examples/multi/strings.c -o examples/multi/strings.obj
    i686-w64-mingw32-gcc -c examples/multi/main.c    -o examples/multi/main.obj
    cargo run -- examples/multi/main.obj examples/multi/math.obj examples/multi/strings.obj \
        -dll examples/multi/mathlib.dll -out examples/multi/multi.exe

# 別 .obj で定義されたグローバル変数を参照する (.data リロケーションの確認)
global_var:
    i686-w64-mingw32-gcc -c examples/global_var/counter.c -o examples/global_var/counter.obj
    i686-w64-mingw32-gcc -c examples/global_var/main.c    -o examples/global_var/main.obj
    cargo run -- examples/global_var/main.obj examples/global_var/counter.obj \
        -out examples/global_var/global_var.exe

# 未初期化グローバル配列 (.bss) の読み書き (RawSize=0 / VirtualSize>0 の確認)
bss:
    i686-w64-mingw32-gcc -c examples/bss/storage.c -o examples/bss/storage.obj
    i686-w64-mingw32-gcc -c examples/bss/main.c    -o examples/bss/main.obj
    cargo run -- examples/bss/main.obj examples/bss/storage.obj \
        -out examples/bss/bss.exe

# dllimport あり (IAT 直接) と なし (.dlljmp スタブ) の両方を使う
jmp_stub:
    i686-w64-mingw32-gcc -shared -o examples/jmp_stub/jmplib.dll examples/jmp_stub/jmplib.c
    i686-w64-mingw32-gcc -c examples/jmp_stub/main.c -o examples/jmp_stub/main.obj
    cargo run -- examples/jmp_stub/main.obj \
        -dll examples/jmp_stub/jmplib.dll -out examples/jmp_stub/jmp_stub.exe

# 生成ファイルを削除する
clean:
    rm -f examples/multi/main.obj examples/multi/math.obj examples/multi/strings.obj examples/multi/multi.exe examples/multi/mathlib.dll
    rm -f examples/global_var/counter.obj examples/global_var/main.obj examples/global_var/global_var.exe
    rm -f examples/bss/storage.obj examples/bss/main.obj examples/bss/bss.exe
    rm -f examples/jmp_stub/main.obj examples/jmp_stub/jmp_stub.exe examples/jmp_stub/jmplib.dll
