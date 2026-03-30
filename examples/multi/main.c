// main.c — エントリポイント
// math.obj / strings.obj の関数 + mathlib.dll の power() を組み合わせるデモ。
// 複数の .obj ファイルと DLL を同時にリンクする例。

#include <windows.h>

// GCC は main() の先頭に __main() を呼ぶ。MinGW CRT なしでリンクするためダミー定義。
void __main(void) {}

// 他の .obj で定義された関数の宣言
int add(int a, int b);
int multiply(int a, int b);
void build_message(char* buf, int a, int b, int sum, int product, int pw);

// mathlib.dll からインポートする関数の宣言
__declspec(dllimport) int power(int base, int exp);

int main(void) {
    int a = 3;
    int b = 4;

    int sum = add(a, b);          // math.obj の add()
    int product = multiply(a, b); // math.obj の multiply()
    int pw = power(a, b);         // mathlib.dll の power()

    char msg[128] = "";
    build_message(msg, a, b, sum, product, pw); // strings.obj の build_message()

    MessageBoxA(NULL, msg, "multi-obj + DLL link demo", MB_OK);
    ExitProcess(0);
    return 0;
}
